# rules-tests — dogfood notes

## tend2

Six checks, six evidence files, one `tend2 verify` run to stamp them. The loop file is read-only for
the worker, so the decisions below have nowhere on the map to land and the conductor has to carry
the ones worth keeping into `## Tried`.

The shape that worked: write the fixture pair first, then the test that reads its expectation off
the fixture rather than off the detector, then the detector. Before the detectors were registered,
every `fire` test failed and every `silent` test passed — which is the right way round, and worth
checking deliberately, because a silent test alone proves nothing about a rule that does not exist.

## What building the test-weakening set found

**A rule that reads test files makes other rules noisier, and that is the truth arriving.** T1's own
fixture drops a case, and a case that goes takes its assertions with it, so T2 now reads the same
change. `tests/rule_t1.rs` used to assert "two findings and nothing else"; it now partitions them
and says the only other reading may be T2 on the file that shrank. That is a stronger claim than the
one it replaced, and it came from weed judging its own diff: the first run reported a T2 error on
`tests/rule_t1.rs` because the first attempt at that edit dropped an assertion to make room.

**Attributing a number to the word that owns it beats reading the nearest name.** T4 has to tell
`toBeCloseTo(0.3333, 4)` from `Total(1, 2)`, and the first version took the name immediately to the
left of each number. That works everywhere except Go, where the tolerance sits outside the call it
belongs to — `math.Abs(Ratio(1, 3)-0.3333) > 1e-4` puts `Ratio` between `Abs` and the number. The
rule now takes the last name to the number's left that says anything about slack at all, and, where
nothing before it does, the first one after it — because a duration is as often written with its
unit behind it (`50 * time.Millisecond`) as in front.

**Nearness is spelled both ways, so the number decides.** One framework's `closeTo(value, delta)`
takes a magnitude and another's `toBeCloseTo(value, digits)` takes a count of digits, and the same
word carries both. Rather than name the frameworks, the vocabulary marks that word ambiguous and the
rule reads the number: a slack is written with a fraction or an exponent, a count of digits is a
whole number. Widening is `>` for the first and `<` for the second.

**A weakened error assertion is a set that shrank.** Scoring specificity per line was fragile — the
input string in `expect(() => parse("a;b")).toThrow("empty")` counts as a message unless you know
which argument belongs to which call. Comparing the two sides as sets fixes it without any parsing:
what appears on both sides cancels, a pair that lost qualifiers and gained none was weakened, and a
pair that gained any is a different claim weed does not rank.

**The substrate does not outline a Go type.** M1 resolves a double named for a type — `MockFormatter`
stands in for `Formatter` — against what the changed production files define, and `tilth-core`'s Go
outline carries imports and functions only, so `type Formatter interface` was invisible. The reader
seam already supplements the substrate for Rust attributes, and it now reads Go type declarations the
same way, single and parenthesised, so every rule that asks what a file defines gets the same answer.
That is a seam change, not a rule workaround: the outline was incomplete, and it was incomplete for
everyone.

**weed reports its own fixtures again, louder.** `weed check` over this loop's diff is clean of
errors and carries 285 warnings, all of them on `fixtures/adversarial/`: T5, because the loop's
detection table names `fixtures/` as a place recorded expectations live and every fixture moved
alongside production code; M1, because the M1 fixtures are, by construction, tests that mock the unit
under change. Both are the rules being right about specimens they should never have been pointed at
— the same gap the rules-block loop left open, and the same fix would close it: a per-path rule
exclusion in `weed.toml`. It is worth noting that T5 reports one finding per expectation file, so a
repository with a large snapshot directory gets a warning per snapshot. Whether that should collapse
into one finding per change is a calibration question, not a detection one.

## Left standing, and not this loop's to close

`cargo test --release` fails `tests/panic_hook.rs` — five of its seven cases — because the door the
hook is proven through is `#[cfg(debug_assertions)]` and a release build does not carry it. Nothing
in this loop touches `src/main.rs` or that test, and every other test file passes on the release
profile, so this is standing breakage in the CI `latency` job rather than something the test rules
introduced. It belongs to whoever owns the panic hook: either the door survives into release, or the
tests that need it say they are debug-only.

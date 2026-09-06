# dogfood — weed (the project loop's own three checks)

## tend2

The three checks left on the project loop are the ones no module loop could
own: the catalogue matrix crosses every rule, the self-check crosses every rule
against weed's own tree, and the latency budget crosses every rule against a
clock. Each of them is a claim about the whole, and the map is the only place
the whole is written down. That is an argument for keeping a few checks at the
root rather than pushing everything onto children.

`tend2 verify --force --expect-payload` re-runs all three. The latency budget
builds the release binary the first time, so the first verify on a cold target
directory takes minutes and every one after it takes ten seconds. Worth knowing
before you decide the gate has hung.

## What the checks found, which is the point of writing them

The self-check was supposed to be a formality and was not. `weed check --base
<root commit>` on weed's own tree came back with 36 block-level findings, and
every one of them was weed being right:

- 34 of them were weed reading its own adversarial fixtures, which are written
  to look dishonest. `[scope] specimens` is the mechanism cape-town ruled on for
  exactly this, and it had been shaped on `rules-prod` and never turned on in
  weed's own `weed.toml`. A capability nobody uses on the repository that built
  it is a capability nobody has tested in anger.
- One was a work marker in the first line of `src/core/rules/scan/r3.rs`, the
  file that implements the rule about work markers. `s1.rs` says the three
  marker words are "written here and nowhere else in this repository", and that
  sentence had quietly stopped being true. Reworded, and the invariant holds
  again.
- One was `weed.toml` itself, because a guardrail file read from the root commit
  is a guardrail file arriving. That one is an allowance with its reason in the
  file, and `tests/self_check.rs` carries a third test that bounds it: an
  ordinary edit to `weed.toml` still blocks. An allowance nobody has drawn a
  line around is a hole, and the line belongs in a test rather than in a comment.

The latency budget found the real bug. Fifty files with a twenty mebibyte file
among them took **86 seconds**, against a budget of two. It was not a rule: with
every rule turned off the run took the same 86 seconds, because the face outlines
every side of every changed file before any rule asks it to, and the parser
scales at about four seconds a mebibyte on a file that is not really code. weed
now stops reading a file as code at the same mebibyte G2 already calls
unreadable, and the worst case answers in 1.2 seconds. The lines are still read,
so nothing hides in a big file: `tests/rule_g2.rs` plants a credential past the
cap and X1 still finds it.

The lesson is the one the loop's own narrative already states and I nearly
missed: a budget is a detector. Nobody would have found that eager outline by
reading the code, because it is correct code doing exactly what it says.

One thing the cap leaves open, named rather than left in the dark: calibration
and the recall campaign were measured before it existed. A source file past a
mebibyte in a real commit of the five garden repositories would now be judged by
its lines alone, and no number in `docs/calibration-2026-09.md` was re-measured
against that. The corpus is pinned, so the next run settles it; the change is
recorded here so nobody has to work out later why a figure moved.

## The matrix, and what "the expected result set" has to mean

Asserting that a fire fixture reports its rule is nearly free and nearly
worthless: most fixtures also trip a neighbour, and a matrix that shrugs at the
neighbours cannot tell a rule that fires from a rule that fires on everything.
So each cell runs under a `weed.toml` that leaves one rule on, written on top of
whatever law the fixture itself wrote, and the assertion is set equality: the
fire cell reports that rule and nothing else, the silent cell reports nothing.
That turned up nothing broken, which is worth saying: the rule loops built their
fixtures cleanly enough that 200 cells came back exactly right the first time
the matrix asked the strict question.

Two fixture kinds needed the harness to grow, and both are the same shape of
problem: weed refuses to carry what it judges. R3 reads git's own record of when
a line was written, so its fixtures state a commit date in `before/.weed-date`
rather than hoping the wall clock cooperates. G2 judges bytes that are not text,
so its fixtures write `{{weed:binary}}` and the harness expands it, the way
conflict markers and credentials were already handled. Following the existing
placeholder convention meant neither one needed a new argument.

`B1` is the awkward cell. `weed bite` runs whatever command the caller names, so
a fixture in a language is only real if that language's runner really runs it:
the matrix drives node, the interpreter, cargo and go, and the CI workflow pins
all three non-Rust toolchains rather than taking whatever the image ships. A
machine without one of them fails the test rather than skipping the language,
which is the right way round.

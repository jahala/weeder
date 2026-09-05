# rules-block — dogfood notes

## tend2

`tend2 verify` stamped all six checks in one run once the suite was green, and each stamp carries
the hash of the test file it was proved by, so a later edit to a test un-stamps its own check. That
is the property that made it safe to write the fixtures before the detectors: nothing could be
claimed until the runner agreed.

The one friction: the loop file is read-only for the worker, so a decision made while building, the
one below about weed refusing its own hooks, has nowhere on the map to land. It goes here instead,
and the conductor has to carry it into `## Tried` by hand.

## What building the block set found

**weed refuses its own installation, and that is the rule working.** `weed guard install` writes
three hooks into `.githooks/` so a clone gets them, and `.githooks/*` is a guardrail path, so the
first commit and the first push that carry those hooks are both C1 findings. The pre-commit hook
runs `--strict`, which reports allowances rather than honouring them, so there is no trailer that
gets a project past it.

This is correct: an agent rewriting `.githooks/pre-commit` to `exit 0` is exactly what C1 exists to
catch, and weed cannot tell that edit from the install. Adopting weed is therefore one deliberate
commit and one deliberate push that go around the gate:

```
weed guard install
git add -A && git commit --no-verify -m "weed guard installed"
git push --no-verify
```

Everything after that goes through. `tests/guard_commit.rs` and `tests/guard_push.rs` now build
their repositories that way, so the adoption step is written down in running code rather than in
prose somebody has to find.

**A masked line loses more than it hides.** The first version of S1 read the do-nothing body test
off a line with comments *and* string literals blanked, which turned `return \`${title}\n${RULE}\``
into `return`, a function returning something read as a function returning nothing. The G1 silent
fixture caught it, not S1's own. The mask now offers three views instead of one, and the body test
takes the view that keeps literals: a body that returns a literal returns something. S1's neighbour
fixtures gained a `return "1.4.0"` in every language so the class stays covered by its own rule.

**The lockfile exemption was redundant, so it went.** X1's entropy path was going to skip files
classified as manifests, on the grounds that recording opaque digests is a lockfile's whole job. It
turned out the name gate already does that work: no lockfile assigns its digests to anything that
reads as a key, a secret, a token, a password or a credential. A guard that can never fire is dead
code, so the exemption came out and the fixture proves the silence the name gate gives.

**weed carries no credential, and it does carry its own bad fixtures.** Running `weed check` over
this loop's own diff reported the X1 fixtures: files full of `AKIA…`, `ghp_…`, a key block and a
signed token, which is a hazard on its own terms, every scanner in the world reads a public
repository. Those now go through the harness the way conflict markers already did: a fixture writes
`{{weed:forge-token}}`, and `tests/common/mod.rs` keeps the stamp and the tail as two strings that
are only joined on the way into the temp repository. Neither half is a credential, so the repository
holds none. The same rule pushed S1's own doc comments to stop writing the three work markers
outside the one string literal that lists them.

What is left when weed judges weed is the T3 and S1 fixtures: files that are meant to look like a
dishonest change, reported as one. That is the rule being right about data it should never have been
pointed at, and fixing it needs a way to say "these paths are specimens", a per-path rule exclusion
that `weed.toml` does not have yet. Left for calibration; it is not something this loop can close by
editing a fixture, because a fixture that stops carrying a skip stops being a fixture.

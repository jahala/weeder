# recall — dogfood notes

Notes from working the recall loop on 2026-09-06, which built `cargo xtask
mutate`. One file per loop; the conductor folds these into `docs/dogfood.md`.

## tend2

The work order was enough to start from without reading anything else: the goal,
the two checks with their evidence paths, and the exact `tend2 verify` line that
decides. Knowing the gate command up front changed how the evidence scripts were
written, they were built to be run by that command rather than by hand, and the
first run of the real gate was the fourth or fifth run of the scripts.

`--force --expect-payload` is a good shape. The payload hash meant the loop file
could be read as the whole brief with no doubt about whether it had moved under
me, and the stamps landed only from the verifier, so there was never a moment
where a checkbox and the truth could disagree.

Two things worth knowing for the next worker.

Both checks run the same two-minute campaign, so `tend2 verify` takes about four
minutes on this loop. That is the honest cost of two claims that each have to be
measured rather than read, but it means a verify is not something to run idly,
and a loop whose checks share an expensive measurement will feel it. A runner
that could hand one check's artefacts to the next would halve it; nothing in
tend2 offers that today, and inventing a cache between them would have been a
staleness bug waiting to happen, so both were left to measure for themselves.

The `Tried` list stopped one wrong turn before it started: "one mutation per
case, never several" is exactly the thing a tired implementer trades away for
speed, and having it written on the map made it non-negotiable rather than a
judgement call at 2am.

## What the loop shape did to the work

The check text, "every miss named", did more than the goal sentence did. It
forced the campaign to keep the before and after of every miss on disk and to
name each one by repository, commit and site, which is what turned a number into
something anyone can argue with. Five of the six things that were wrong in the
first run were found by reading that list, not by reading the totals.

## The check that caught its own report

A later session found `misses()` in the report writer returning an empty list
whatever it was handed, so the file said "No case was missed" over a run that
missed fifty-two. Both evidence scripts caught it, from opposite directions:
`recall.sh` compared the run's own json against the file and listed every miss
the prose had dropped, and `recall-bar.sh` refused a count of named misses that
did not equal the count of misses. Neither check reads a number the campaign
wrote down and believes it, and that is what made a silent hole in the report
loud instead of invisible.

Worth copying into other loops: when a check says "and every X is named", have
the evidence script recompute the set of X from the run and diff it against the
document, rather than asserting the document has a section with that heading. A
heading is cheap to satisfy; a set is not.

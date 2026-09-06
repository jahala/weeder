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

## The pinned run, 2026-09-06

The third check moved the campaign off "whatever the branch says today" and onto
the corpus calibration judges. Three things are worth passing on.

The corpus was already data, in `docs/calibration/corpus.toml`, and the campaign
had its own copy of the same idea in Rust: seven repositories, five of them named
by a path on the conductor's laptop, each with the languages it writes declared
beside it. Deleting that list was most of the work. What replaced it reads the
corpus file, fetches each entry at its pin, and counts the languages off the tree
at the pin, so a corpus entry is a name, a source and a sha and nothing else. The
languages had to be counted rather than declared, because a corpus file has no
column for them and a purpose-built history in a test has no entry in a table.

The Go pair the garden has no Go for now sits in `docs/calibration/corpus-go.toml`,
read the same way. One shape, two files, and `--corpus` reads any other, which is
what let the evidence script measure a history it built in a temp directory.

The measurement moved, which is the point of taking it again. tilth's window had
been read off `origin/main`, 397 commits; the pin carries 529. copeca had been
read off a branch with hundreds of commits and its pin carries 27. Pinning is not
free, it changes which commits the cases are planted in, and the figures with it.

## The rule map moved under the campaign

`weed rules` said C3 where the injector said C1: the C1 split landed after this
loop's first two checks were stamped, so the campaign was planting a workflow
edit and calling it C1, which the binary now reports as C3, and C1 in the report
read 0.0 percent in all four languages. The evidence scripts read the catalogue
off the binary rather than from a list of their own, which is why this surfaced
as a bar failure instead of a quiet gap.

For the next worker: a campaign that names rules by id has a dependency on the
catalogue that nothing in the build checks. The two evidence scripts here demand
a row for every check rule the binary carries, which catches a rule that arrives;
neither catches a rule whose meaning changes under its id. Only a run does.

## The verifier's two minutes

`tend2 verify` runs each check with a two-minute timeout it does not document and
no flag reaches: `spawnSync(..., { timeout: options.timeoutMs ?? 12e4 })`, and
nothing on the command line sets `timeoutMs`. A check killed at 120 seconds is
reported as a failure whose detail is the last 300 characters of its output,
which for a campaign that prints only at the end is the last line of a cargo
build. It reads exactly like a check that ran and disagreed.

That is what this loop hit. The full campaign took 166 seconds, so both of the
earlier checks failed on the clock rather than on a number, and the third failed
because the two killed shells left their campaigns running: `spawnSync` kills the
shell it started and not the processes under it, and three campaigns checking out
commits in one cache collided on `index.lock`.

Two things came out of it. The campaign now walks each repository's window from
four places at once, each in a working copy of its own, which took the run from
166 seconds to 61 and left the figures where they were: a case costs a checkout
and a run of the binary, so the campaign waits far more than it computes, and the
slice count is fixed at four rather than read off the machine so that two
machines plant the same cases. And a case now writes the index only when it
brings a file git has never been told about, since a diff against a commit reads
the working tree for everything git already knows.

For tend2: a per-check timeout that cannot be raised puts a ceiling on what a
check may measure, and the failure it produces is indistinguishable from a real
one. Two things would have saved an hour here — the timeout in the failure detail
("killed after 120s" rather than a truncated tail), and a `--timeout` flag for
checks that measure something real. Killing the process group rather than the
shell would have saved the third check from the wreckage of the first two.

# calibration — dogfood notes

## tend2

The loop file carried the whole job. Its narrative decided three things the
checks alone would have left me guessing at: that the repositories are named
with paths, that the bar is pooled rather than per repository, and that
classification is a person's job and the script only checks the arithmetic.
That last line is the one that shaped the design most: it is why the
classifications live in `docs/calibration/judgements.toml` and the report is
generated from measurement plus ledger, rather than being a document somebody
edits and a script then believes.

`## Tried` earned its keep. Four of its five lines changed what I built, and the
fifth, cape-town's ruling on specimen exclusions, is the one thing on this loop
I did not land. It is not one of the four checks, it changes a detector's config
surface, and it would have moved the numbers this loop exists to record. It is
named here rather than left in the dark.

`tend2 verify --force --expect-payload` is a good gate to work against: the
payload hash means I could not have quietly reworded the goal to fit what I
built.

## pleach

The work order was complete enough to start from cold. The one thing it does
not say, and could, is what the worker may change outside its own files. This
loop needed a `.cargo/config.toml` for the `cargo xtask` alias, and
`scripts/check/tilth-core-dep.sh` refused any cargo config beside the manifest.
Tightening that check from "no config" to "no table but `[alias]`, and no
`paths` key" keeps the guarantee it was written for and lets the alias exist,
but it changes another loop's evidence file, so **tilth-core c2 needs its stamp
re-earned after this lands**. A work order that named the evidence files a node
may touch would have let me raise that before writing code rather than after.

## weed, on itself

Judging 635 real commits found three defects worth naming, and they are in the
report under `Where weed was wrong`:

- **X1** reads a `name = value` shape wherever it finds one, so a css class
  attribute (`class="detail-meta__key">`) and a line of markdown prose about
  credentials both read as an assigned secret. Seven of this run's nine false
  positives are that one shape, across four repositories. It is the single
  highest-value fix on the rule side.
- **S1** calls a Python `Protocol` method with an ellipsis body a stub. In
  Python that ellipsis is how a protocol declares a method's type; there is no
  implementation to finish.
- **T1** counts statically declared test cases, so a case generated inside a
  `for` loop reads as a case that disappeared.

None of them was fixed here. Calibration measures; a rule that changes while it
is being measured measures nothing, and each of these belongs to the loop that
owns its rule. The bar is met without them: 1.42 percent pooled, and 0.31
percent if X1 alone were fixed.

The friction the gate creates is almost all C1: 42 of 71 blocks are a GitHub
workflow edit, every one of them real. Whether that friction is wanted is a
product decision this loop can only put a number on.

## The recalibration, 2026-09-06

The corpus is pinned now: each repository is a url and a full sha, and the run
fetches that commit and walks the window ending at it. Three things fell out of
doing it.

The five garden repositories all have a GitHub remote whose default branch was
exactly what the local checkouts held, so pinning changed no number: 635 commits
judged before and after. That will not be true next time, and it is the point , 
the pins are now the thing that has to be edited for the report to move.

Fetching a bare sha works from GitHub and from a path on this machine alike, so
the scratch is one `git fetch <source> <sha>` and one `update-ref`. No clone of
a branch, no guessing at which branch a checkout is on, and `default_ref` went
away with the guessing.

Measuring the C1 split needed the first run's findings, and a report's tables do
not carry the file a rule fired on. The pre-split code is still in git history,
so the record was made by running it: `git archive a999e23`, `cargo xtask
calibrate --findings`, and its precision half came out byte for byte identical
to the report in the tree, which is what makes `docs/calibration/first-run.toml`
the first run's own record rather than a reconstruction. 332 findings, 69 of them
C1 on workflow files, and all 69 are C3 warnings today.

The rule fixes rules-prod and rules-block landed show up plainly: 71 blocks down
to 25, nine block-level false positives down to two, and 58 of the first run's
findings are reported by nobody now. Pooled share 0.31 percent.

What is still not reproducible is the recall campaign. `cargo xtask mutate`
picks its own branch per repository, tilth at `origin/main`, tend2 at
`origin/landing-rewrite`, and reads whatever those say today, so the miss list
can move between two runs over what is nominally the same corpus. Pointing it at
the pinned corpus would change which commits it walks and so the recall figures
themselves, which is the recall loop's measurement to re-take, not this one's to
quietly alter.

A second run of this node was refused at the delivery gate rather than by a
check: the secret scan found an AWS access key id in the worker's scratch
journal and stopped the whole run. The key was the one AWS prints in its own
documentation, and it reached the journal because the journal quotes the commit
message of a corpus commit that X1 blocked. Quoting it again here would fail
this delivery the same way, which is the point. A calibration worker's notes
will always carry
credential-shaped strings, because credential-shaped strings are what the rule
it is calibrating finds; the scan cannot tell a quoted finding from a live key,
and neither can weed, which is why the judgement beside that commit reads
acceptable. Nothing here needs a scanner that is cleverer. What it needs is for
the journal to redact what it quotes.

The same run left 836 MB under `.loop-scratch/`, a whole second copy of the
repository, taken so the first run's tree could be read, and the collector
staged all of it, because this repository stopped ignoring that directory after
P8. `weed check --base HEAD --strict` on the delivery exited 3 on that copy: it
carries weed's own X1 prefix table and an allowance with no reason written on
it, both of them fine where they live and both of them findings once they are
pasted into a diff. A scratch directory that is collected is not scratch.
Either the collector skips it or the loop stops calling it disposable.

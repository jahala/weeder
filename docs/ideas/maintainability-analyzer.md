# A maintainability analyzer that is deterministic and still worth reading

An idea, not a loop. It came out of a proposal to widen weeder into architecture
and maintainability judgement (2026-09-11). The first three parts of that
proposal, resolved dependency graphs, declared feature interfaces and dependency
cycles, are weeder's and are shaped separately. This file is about the rest: a
tool that says where a codebase is getting harder to change, why, and what to
do, in a way that changes how agents write and maintain code.

## The claim it makes, and the one it never makes

The analyzer says: this part of the code has become more expensive to change,
here is the measurement, here is the comparison that makes the measurement a
change rather than a state, here is the evidence, here is an intervention whose
effect can be measured afterwards. It never says the code is bad, and it never
blocks anything. weeder blocks; this diagnoses.

A finding is a record with five fields: the subject (a function, a file, a pair
of features), the measure, the baseline the measure is read against, the
evidence (commits, sites, lines), and the intervention with its own success
measure. A finding without any of the five is not written.

## Deterministic inputs

Everything the analyzer reads is a fact about a repository at a named commit or
over a named window of history. Same inputs, same version of the analyzer, same
bytes out, in any locale or process, the rule weeder already keeps.

Structure, per file and per function, at a commit: outline, resolved imports,
callers, decision points (branches, loops, boolean operators, early returns),
nesting depth, size, and a duplication fingerprint (normalised token shingles,
winnowed, so a block copied with renamed variables still matches). These come
from tilth-core, which already reads outlines, imports and callers; the counts
and the fingerprint are the additions tilth needs to make. Results are cached by
blob hash and tilth version, so a file read twice is read once.

History, over a window that ends at a pinned commit: which files and which
functions each commit touched (function level comes from diffing outlines, not
from guessing at hunks), renames followed, and three classifications made by
shape rather than by message: a revert (bytes restored to an earlier state), a
fix that lands within N commits on the same function, and a bulk edit (many
files, formatting-only or mechanical). Bulk edits are set aside before anything
is counted, or every reformat looks like coupling.

Declarations: the repository's own word for its parts, `[deps] layers` and
entries in weeder.toml, ownership if declared. The analyzer reads them; it does
not invent them.

## Measures, each with a baseline

No threshold is picked by feel. Each measure is a rate read against a baseline
the report prints beside it, so a reader sees what "high" means for this
repository.

- Spread: how many declared parts one commit touches, as the entropy of touched
  parts. Baseline: the repository's own distribution. A feature whose commits
  keep spreading across parts is a feature whose boundary is in the wrong place,
  or is not the boundary the declarations say.
- Observed coupling: for a pair of parts, lift, the rate they change together
  divided by the rate chance would give, with a minimum support so two
  coincidences do not make a finding. Read against static coupling: a pair that
  co-changes with no import between them is coupled through something imports
  cannot see, a data shape or a convention, and that is the interesting case.
- Structural drift, per function: decision points and nesting over the window,
  as a series, not a snapshot. Only a function that has grown across several
  changes is a finding; one large commit is a design, not a drift.
- Duplication that costs: a fingerprint present in k places, and whether those
  places co-change. Copies that change together are one piece of logic paying k
  times; copies that never change together may be independence worth keeping,
  and the report says which it saw.
- Hotness: function-level churn, read against the function's size, so a large
  file with one busy function names the function.

There is no composite score. The report is a ranked table per measure with the
evidence attached, and a short section that names the subjects appearing in more
than one table, which is where the money usually is. If a single number is ever
wanted, its formula is versioned and printed.

## How it earns trust

The same way weeder did: on real history, against a baseline, with the result
published. The garden already pins seven histories under weeder's
`docs/calibration/`. The test is predictive, not a nod: rank the first half of
each history, then measure whether the ranking says where the second half's
fixes and reverts landed better than the dumb baseline, churn times size, did.
Each measure ships only if it beats the baseline on its own; one that does not
is dropped, and the report of the experiment says so. The kill criterion for the
whole idea is the same sentence: if the combined ranking does not beat churn
times size on the garden's own histories, this stays a document.

## How it changes what agents do

Advice does not change an agent's behaviour; inputs and gates do. The analyzer
plugs into three of them.

1. Findings become declarations weeder enforces. An observed-coupling finding
   proposes a boundary in weeder.toml's own words, a layer, an entry path; a
   person accepts or edits it; from then on weeder's D rules refuse the import
   that would breach it. Diagnosis becomes contract becomes gate, and the agent
   meets the gate at commit time, which is where behaviour bends.
2. Findings become loops. tend2's `discover` reads the ranked report and
   proposes a loop per finding with the intervention as its goal and the measure
   as its check: duplicate count of this fingerprint is one; this pair's lift is
   under two over the next twenty commits. The check is earned by the analyzer
   re-run, so the loop closes on evidence rather than on prose.
3. Findings become context at the moment of writing. When an agent's harness
   opens a file the report names, the hook weeder already answers (PreToolUse,
   Stop) can carry one deterministic line from the report: this function grew in
   five of its last eight changes; the open loop asks for it to shrink. Warn,
   never block, and only where a finding exists, so the line is rare and read.

The report itself is SARIF beside its own JSON, so a pull request shows a
finding inline the way weeder's land, and an optional `explain` face may hand a
finding to a model to be put in words. The model reads findings; it never
produces one.

## Shape

Three faces: `scan`, the whole tree over a window, writes the report; `diff`,
one change against its base, says whether a named hotspot got worse, advisory;
`declare`, the boundary proposals in weeder.toml form. A garden bed of its own,
with a manifest, a skill and the hooks above, fitted the way weeder was.

The order of building is the order of dependence: the tilth-core additions
first, because every bed can use them and they pay off whether or not this
ships; then the history reader and the measures as an experiment script over
the pinned corpus; then, only past the kill criterion, the bed.

## What it must not become

A score that reads as a verdict. A threshold with no baseline printed beside it.
A gate. A style judge. A tool that names a vendor. A report that says a thing is
worse without saying against what, over which window, and where to look.

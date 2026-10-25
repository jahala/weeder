# A maintainability analyzer that is deterministic and still worth reading

An idea, not a loop. It came out of a proposal to widen weeder into architecture
and maintainability judgement (2026-09-11), and was tightened the same day by a
second reading that caught five overreaches; those corrections are in here. The
first three parts of that proposal, resolved dependency graphs, declared feature
interfaces and dependency cycles, are weeder's and are shaped separately. This
file is about the rest: a tool that names plausible maintenance problems worth
investigating, with evidence and explicit uncertainty, and then measures whether
the intervention someone chose helped.

## The promise, kept narrow

The analyzer says what it observed, then offers a qualified interpretation, and
keeps the two apart. It never says the code is bad, never says a change was
costly (nothing here measures cost), and never blocks. weeder blocks; this
diagnoses.

A finding carries: the subject (a function, a file, a pair of parts), the
observation with its measure and the baseline the measure is read against, the
evidence (commits, sites, lines), the uncertainty (what could not be resolved or
matched), and the interpretation, marked as such. An intervention is optional: a
hypothesis with its prerequisites and how it would be validated. Where the
evidence supports only an investigation, the finding says so and stops.

Each interpretation names its other readings, because every measure here has
one. High spread can be a legitimate cross-cutting requirement. Co-change with
no import between the parts can be a coordinated release, or an import the
resolver could not see. Copies that change together may need one home, or may
implement separate contracts. One large commit can be a design; several small
ones can be a drift, or the reverse.

## Deterministic inputs

Everything the analyzer reads is a fact about a repository at a named commit or
over a named window that ends at a pinned commit. Same inputs, same version of
the analyzer, same bytes out, in any locale or process, the rule weeder keeps.
Exclusions, unresolved relationships and uncertain identities are reported
beside the measurements, never silently dropped.

Structure, per file and per function, at a commit: outline, resolved imports,
callers, decision points, nesting depth, size. tilth-core reads outlines,
imports and callers today; the counts are small additions, and duplication
fingerprints wait until the experiment says they earn their place. Syntax facts
that depend on the file alone are cached by blob hash and tilth version.
Resolved imports and callers depend on the path, the repository state and its
configuration too, so they are cached by those as well or not at all.

History, over the window: which files and which functions each commit touched.
Function identity is body-aware: an outline can stay the same while the body
changes completely, so functions are matched across commits by body similarity
as well as name and place, with extractions, moves and uncertain matches
recorded rather than guessed, or a refactor reads as one hotspot deleted and an
unrelated function born. Renames are followed. Three shapes are classified by
what they literally are: a restoration (bytes returned to an earlier state), a
rapid rework (a further edit to the same function within N commits), and a bulk
edit (many files, formatting-only or mechanical), which is set aside before
anything is counted. A restoration is not called a defect and a rework is not
called a fix; the reader is told what was seen.

Declarations: the repository's own word for its parts, `[deps] layers` and
entries in weeder.toml, ownership if declared. The analyzer reads them; it does
not invent them.

## Measures, each with a baseline

No threshold is picked by feel. Each measure is a rate read against a baseline
the report prints beside it, so a reader sees what "high" means here.

- Spread: how many declared parts one commit touches, read against the
  repository's own distribution.
- Observed coupling: for a pair of parts, lift, the rate they change together
  divided by the rate chance would give, with a minimum support, read against
  static coupling from the import graph. The interesting case is a pair that
  co-changes with no import between them, and the report says whether an
  unresolved import could explain it.
- Structural drift, per function: decision points and nesting over the window,
  as a series. Read with the commit shapes beside it, since one large commit and
  several small ones can each mean either thing.
- Duplication, once fingerprints exist: identifier-normalised (winnowing alone
  does not make renamed variables match), keeping scope relationships and
  meaningful literals so unrelated logic does not share a fingerprint; and
  whether the copies co-change.
- Hotness: function-level churn read against the function's size.

There is no composite score in the report. Each measure is a ranked table with
its evidence and its uncertainty. Subjects that appear in more than one table
are listed together as candidates for one investigation.

## How it earns trust

On real history, against baselines, with the result published, and without
rewarding churn twice: a high-churn function predicts another edit nearby, and
an outcome that calls that edit a fix would hand the baseline its own answer.
So the outcomes are kept apart and each is reported on its own:

- future churn, which is useful to predict and is not maintenance difficulty;
- corrective changes verified independently of churn: restorations, and
  changes the garden's own records mark as corrective, the calibration ledger's
  judged blocks and the loops' Tried lines that name where a repair landed;
- where the records allow it, effort and regressions on matched maintenance
  tasks, which the garden has in pleach receipts and the tend2 ledger.

The experiment ranks on several chronological cutoffs, with repositories held
out from any tuning, against three baselines: churn alone, size alone, churn
times size. It reads precision at a fixed review budget, of the five findings a
person can inspect, how many were useful, and it asks of each measure whether it
adds information beyond the baselines rather than whether it beats them alone.
The separate ranked lists are compared under one shared budget, interleaved by
a rule the report states; that is the only place lists are combined, and it is
an experimental rule, not a score.

The kill criterion: if, under a shared review budget, the lists do not beat
churn times size on held-out garden histories on the corrective outcomes, this
stays a document.

## How it changes what agents do

Advice does not move an agent; inputs and gates do. But a metric target moves
an agent the wrong way: "the duplicate count is one", "the lift falls under
two", "this function shrinks" are each satisfiable by an inappropriate shared
abstraction, a change of commit grouping, or logic scattered into helpers. So
the analyzer's findings reach agents in three shapes, none of them a number to
hit.

1. Investigations, not loops. Related findings are grouped into one
   investigation; a person reads it and chooses an intervention, and leaving the
   code alone is a valid choice. Only a chosen intervention becomes a loop, and
   its goal is stated in ownership terms: refund eligibility has one clearly
   owned implementation. Its check is behavioural: a representative eligibility
   change stays within that ownership boundary, preserves behaviour, and
   introduces no forbidden dependency, which the tests and weeder's D rules can
   each hold. Structural measures are supporting evidence in the loop's record.
   Historical coupling is a delayed monitoring signal, read again after the
   fact, never a completion check.
2. Declarations, later and reviewed. A `declare` face proposes boundaries in
   weeder.toml's own words from findings a person has read, never as an
   automatic translation of a statistical relationship. Once accepted, weeder
   enforces them at commit time; the analyzer never enforces.
3. Context at the moment of writing, through the analyzer's own harness
   adapter, not weeder's. When an agent opens a subject the report names, one
   deterministic line describes the observation and points at the open
   investigation. It does not direct the agent to shrink or split anything.

The report is SARIF beside its own JSON, so a pull request shows a finding
inline the way weeder's land, and an optional `explain` face may hand a finding
to a model to be put in words. The model reads findings; it never produces one.

## Shape, and the order of building

Start smaller than the infrastructure. First a reproducible experiment over
what exists: import structure and callers from tilth-core, body changes from
diffs, history from git, on the seven pinned histories under weeder's
`docs/calibration/`. Add an extraction capability only when a measure the
experiment needs is missing it, and drop a measure that adds nothing over the
baselines. Then, past the kill criterion, a bed with three faces: `scan`, ranked
evidence with explicit uncertainty over a window; `diff`, observations about
what changed at established hotspots in one change, advisory; and later
`declare`, for reviewed boundary proposals. A manifest, a skill and the adapter
above, fitted the way weeder was.

## What it must not become

A score that reads as a verdict. A target an agent can satisfy without
improving the code. A threshold with no baseline printed beside it. A gate. A
style judge. A tool that names a vendor. A report that says a thing is worse
without saying what was observed, against what, over which window, with what
uncertainty, and where to look.

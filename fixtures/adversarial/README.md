# Adversarial fixtures

One minimal repository history per rule per language, at
`<RULE>/<lang>/<fire|silent>/`. `before/` is committed as HEAD, `after/` replaces
the working tree, and a file `after/` does not carry is a file the change
deleted. `after/.weed-commit` is the message of the commit being prepared, for
the rules that read trailers; it never reaches the tree. An ignore file is
written as `weed.gitignore` and copied in as `.gitignore`: named literally, it
would govern the fixture's own directory and hide its sources from git.

`fire/` is a change the rule must report. `silent/` is its nearest neighbour, the
change that looks like the offence and is not one. A rule with no `silent/`
fixture has not been shown to discriminate.

A rule with more than one shape to prove carries more than one of each, named
for what it holds: T5 has a `silent-code/` for the half where the code moves
without the expectation, T7 has `fire-case/` and `silent-case/` for the
languages whose runners collect a case by the name it is declared under, T1 has
`silent-table/` and `fire-table/` for the suite that moves its cases into a
table and the one that leaves a case behind while it does, C3 has a `promoted/`
whose `weed.toml` names a workflow under `[guardrails] paths`, and R1 has a
`fire-bound/` whose document cites the commands a scan must refuse to run.
Every directory holding a `before/` and an `after/` is a fixture, whatever it is
called, and `tests/determinism.rs` replays all of them.

Every rule the catalogue prints answers for all four languages, `ts/`, `py/`,
`rs/` and `go/`, and `tests/catalogue_matrix.rs` walks that matrix and holds each
cell to its whole result set. A rule that reads paths rather than a language
keeps its extra shapes under one folder named for what it reads: C1 and C3 under
`paths/`, X1 under `prose/` for the files weed has no grammar for.

A fixture whose rule asks git when a line was written states the date its state
was committed on in `before/.weed-date`, in any spelling git reads. The file
never reaches the tree, and R3's fixtures are the ones that carry it: a marker
old enough to report has to have been written long ago, and a repository built
today cannot say that any other way.

A rule on the `scan` face judges the tree rather than a diff, so its fixture has
one state and not two: `before/` alone, committed as HEAD and left in place as
the working tree. `after/` is absent, and the harness knows to stop there.

`tests/common/mod.rs` builds the repository. Nothing here is copied by hand.

## Conflict markers

weed does not carry what it refuses, so no file here opens a line with a
conflict marker. A fixture writes these placeholders instead, and the harness
expands each into seven of its character before the file reaches the temp
repository:

| placeholder            | marker    |
| ---------------------- | --------- |
| `{{weed:ours}}`        | the opener |
| `{{weed:base}}`        | the base side of a diff3 merge |
| `{{weed:separator}}`   | the divider |
| `{{weed:theirs}}`      | the closer |

A label follows the placeholder as it follows the marker, so
`{{weed:ours}} HEAD` becomes the line git wrote. The binary under test reads the
real bytes; only the file on disk here is spelled differently. A separator in
the middle of a line, the one a `silent/` fixture keeps inside a string, is
written out, because that is the case the rule has to let through.

## Bytes that are not text

The same rule applies to a blob. `{{weed:binary}}` is expanded into a run of
control bytes opening with a NUL, which is git's own test for a file it cannot
show a diff of and the one weed asks too. G2's `fire/` fixtures write it, and
this repository carries no blob of its own to prove the point with.

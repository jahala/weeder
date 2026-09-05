# dogfood — scan

## tend2

`tend2 verify docs/tend2/scan.tend2.html --repo-root . --force --expect-payload 0e3b0bbac3a6
--runner 'bash scripts/check/run.sh {evidence}'` stamped six checks in one pass, exit 0, nothing to
argue with. Six evidence files, five of them `cargo test --test <name>` through `run.sh` and one a
shell script, and the runner template handled both without a second flag.

`--expect-payload` earned its keep here. The loop is read-only for a worker, and the payload sha is
what makes that a fact rather than an instruction: the checks are the spec, and a worker who edits
the spec to fit the code is caught at the gate rather than at review. It is the one flag on that
command that changes what "verified" can mean.

Question 1 from `docs/questions-tend2-pleach-2026-09-05.md` is still open and still costs something.
The runner template has to be typed on every invocation, so the difference between a verified check
and an unverifiable one is a flag somebody may forget. A `runner:` line on the map would close it.

## Where the loop's own wording did the work

The check list named four kinds of citation for R1, a path, a command, a flag, a symbol, and asked
for one fixture each. That is what forced the rule to have four authorities rather than one clever
heuristic: the tree answers for a path, the command's own `--help` answers for a subcommand and a
flag, the code answers for a symbol. A check written as "R1 warns on stale documentation" would have
been closable with a regular expression and a list of known-good words, and it would have been
worthless three months from now.

The same wording is what stopped R4 from being a network client. "With no network reached" is
testable, a fetcher on PATH that records being asked, and a scan that never touches it, and the
test is three lines. A check that had said "R4 compares against the registry" would have shipped a
rule that answers differently on a Monday.

## What the rules found on weed itself

`weed scan --refresh-snapshot` against the real registries, run once from a scratch copy, reported
weed's own `clap` six minor releases back, `tempfile` twenty-seven back, and `toml` a whole major
behind. All true. The snapshot was not committed: this loop was not asked to pin weed's own lag, and
a committed snapshot would make every later scan of this repository argue about it.

`weed scan` on weed's own tree reported 201 findings the first time it ran, and every one of them
was read. Most were R1 telling the truth about the wrong repository: this repository's docs describe
tend2, pleach and umbel, and their paths and names were resolved against this checkout. Five
precision passes took it to 93, a citation carrying a placeholder or an elision is a shape rather
than a place; a fragment or a line range is stripped before the path is resolved; `path::symbol` is
two citations under two authorities; a symbol resolves against everything the repository holds
except the prose being judged, so a vendored schema vouches for the field names a doc cites; and a
citation ending in a directory rather than a file is resolved only when its first segment is
something this tree has, which is what takes `and/or` and a branch name out of it.

Two of what is left are true: the docs cite `docs/rules.md`, which `sarif::render` builds a
`helpUri` against and nobody has written, and they cite `tests/speed.rs`, which the calibration loop
has not landed. Finding those on the first run is the rule working.

Two things this loop did not solve, both real:

- weed cannot see which repository a sentence is about. `docs/tend2-seam.md` cites tend2's
  `src/verify/mock.ts` and gets a warning for it. The escape is `[rules] R1 = "off"`; the fix is a
  way for a document to name the tree it describes, and that is a later loop's shape to make.
- R2 reports 17 exports in `fixtures/adversarial/`, all of them true and none of them wanted: a
  fixture is an input to a test, not part of the program. The fix is a `[scan] exclude` list of
  globs a scan does not read, which every linter has and this loop was not shaped to add. Until it
  lands, a repository that keeps sample code in the tree turns R2 off.

## The two binding checks, later

Cape-town added two checks to the loop after milestone 5, and `tend2 verify --expect-payload`
made the new payload sha the thing to satisfy: the same command, a different number, and the two
new checks unstamped until their evidence ran. Nothing about the shaped checks had to be explained
twice.

Writing the exec bound as a test found nothing wrong: the face already handed `exec::run` a program
and an argument array, and R1 resolves a citation against listings it already has rather than by
running anything. What the test adds is the record. Its PATH is a directory of scripts that write
down the arguments they were handed, so the claim is read off the commands themselves: two lines,
`listed-tool --help` and `listed-tool build --help`, and the second one is there because the first
one's help named `build`. Routing the same call through `sh -c` to see the test fail took one line
and the log came back saying `sh -c listed-tool --help`.

The offline check found a real defect. `curl` exiting 6 because a host does not resolve was read as
a registry with no latest release to give, so a refresh with no network wrote a snapshot saying
every registry has released nothing, and the next scan would have called every pin current. A
refresh now tells the two apart: a registry that answered, and one weed never reached. Reaching
none of them is a refresh that did not happen, so weed leaves with 3 and the committed snapshot
stays as it was. The denial the test runs under is `sandbox-exec` on macOS and `unshare` on Linux,
and the refresh coming back unreachable is what proves the denial is real rather than decorative.

`weed rules` grew a network column out of the same check. It is the one place a reader can ask
what the judge may reach without reading the source, and the answer is `none` on every rule but R4.

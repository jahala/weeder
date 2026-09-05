# dogfood — hardening

## tend2

`tend2 verify docs/tend2/hardening.tend2.html --repo-root . --force --expect-payload 119b397a7503
--runner 'bash scripts/check/run.sh {evidence}'` did what it says: five checks, five evidence files,
five stamps, exit 0. Nothing to fight with.

Two things worth passing on.

The five checks are named by evidence path and the runner maps `tests/<name>.rs` to
`cargo test --test <name>`. That is a clean seam, and it also means a check is only as honest as the
test file it points at — `tend2 verify` runs the file and reads the exit code, so a file full of
assertions that cannot fail stamps just as green as one that hunts. Nothing tend2 can do about that;
it is worth saying out loud because the stamp looks like proof and is really a receipt.

Running the same command a second time re-runs all five suites. On this loop that is about forty
seconds, most of it the 20 MiB fixture in `check_hostile` and the property runs in `core_hostile`.
Fine here. A loop whose evidence takes minutes would want to know that `--force` means everything
again, not just what moved.

## The order the checks were worked

The order law had to land before the determinism replay could mean anything — the replay would have
passed on the unsorted `render` too, because a single process's hash order is stable within a run
and G1 reports in file order anyway. Sorting first, then replaying, is what makes the second check
more than a formality. Worth remembering when a loop lists checks that look independent: the map
does not say which one holds the others up.

## What the property test found

The `proptest` check was not a formality either. It found two panics in code that already had tests:
a hunk counter running off the end of `u32` in `parse_diff`, and a slice through the middle of a
character in the diff-header path split written for this same loop, twenty minutes old. The second
one is the useful one — the property test caught a bug in the fix for another check, in the same
session, before it left the worktree.

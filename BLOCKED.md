# blocked — windows, on the half of c2 that reads a run

Two of the three code checks are closed and the verifier stamped them. The third
cannot be closed from a worker's worktree, and the reason is an ordering one
rather than a missing piece of work.

    tend2 verify docs/tend2/windows.tend2.html --repo-root . --force \
      --expect-payload cfbeccf6fc03 --runner 'bash scripts/check/run.sh {evidence}'

    docs/tend2/windows.tend2.html:c1 stamped
    docs/tend2/windows.tend2.html:c2 failed
    docs/tend2/windows.tend2.html:c3 stamped
    docs/tend2/windows.tend2.html:c4 skipped-human

## What is done

- c1, stamped by `scripts/check/platform-gates.sh`. Every `std::os::unix` line
  in `src/`, `xtask/` and `tests/` sits inside a `cfg(unix)` gate that either has
  a `cfg(windows)` twin under the same name or carries a comment saying what
  Windows has instead. `is_executable` and `make_executable` are written twice in
  `src/seams/fs.rs`, with the passage above them saying that whether a file is a
  hook is git's answer and not weeder's: the execute bit on unix, the file being
  there on Windows, where git takes `X_OK` out of its `access` call. The host
  suite is green, `cargo fmt --check` is clean and clippy is clean with
  `-D warnings`.
- c3, stamped by `tests/garden_manifest.rs`. `Cargo.toml`, `npm/package.json` and
  `garden.json` say 0.2.2, and the manifest declares five assets whose urls name
  the v0.2.2 release, the fifth of them
  `weeder-x86_64-pc-windows-msvc.tar.gz`.
- The yaml half of c2. `.github/workflows/ci.yml` carries a `windows` job on
  `windows-latest`, on the same push and pull request events as the ubuntu job,
  with no `if` and no `continue-on-error`, building the workspace and then
  running the same `cargo test --workspace` the other jobs run.
  `scripts/check/windows-ci.sh` reads that out of the workflow and says so.

## What stops c2

`scripts/check/windows-ci.sh` has two halves, because the loop's claim has two.
The first reads the workflow, and it passes. The second reads master's newest CI
run through `gh` and refuses when the windows job is missing, skipped or red:

    run 34464028361 on master (ef7bcbc4c6cb) has no windows job:
    it ran ['check', 'latency'].

That run is master's tip, from before this work. The job can only appear in a run
on master after this change is merged, and merging it is not the worker's to do:
the dispatch prompt says to leave the changes in the working tree and not to
create branches, commit or push, and `AGENTS.md` says not to push, publish or
create anything on GitHub. So c2 is false for exactly as long as the work sits in
a worktree, and becomes true on the first CI run after it lands.

There is no local substitute for that run. The `x86_64-pc-windows-msvc` target is
installed on this machine and `cargo check --target x86_64-pc-windows-msvc` stops
long before weeder's own code: every tree-sitter grammar is built by a C compiler
and that one wants MSVC. The loop says as much when it shapes the check, which is
why the check reads a run rather than a tree.

## What a fix needs

Either of these, and the first is the smaller one.

1. Land this working tree, then re-run the same verify command. Nothing in the
   tree needs to change: the moment master's newest CI run carries a green
   `windows` job, `scripts/check/windows-ci.sh` reads it and c2 stamps. The
   ordering to run it in is push the branch, let CI run, read the `windows` job,
   merge on green, wait for master's own run, then verify. If the first Windows
   run is red, that is this loop working: the release leg failed at a tag once,
   and a red check on a pull request is where it was supposed to fail.
2. Split c2 on the loop into two tiers a worker and a conductor can each close:
   a shape tier that reads the workflow, which a worker can hold up, and a run
   tier that reads the run on master, which only something landing can. This is
   the larger change, it needs the loop reshaped, and it buys only the ability to
   stamp part of a claim earlier.

## What the first Windows run may still find

The suite has never been run on Windows, so this is the honest list of where a
red would come from, for whoever reads it.

- The guard tests write `#!/bin/sh` hooks and let git run them. Git for Windows
  runs a hook it finds through the `sh` it ships, which reads the shebang line.
  `tests/guard_planter.rs` copies the binary to `.plotplot/bin/weeder` with no
  `.exe` on it, on purpose, because the point of that test is a path weeder's own
  install would never write; MSYS starts such a file on its `MZ` magic rather
  than on its name.
- Temp paths. The runner's own `TEMP` names a user directory by its 8.3 short
  name and git answers every question with the long one, so the job pins `TMP`
  and `TEMP` to `runner.temp`, where no component has two spellings.
- Line endings. `.gitattributes` turns every conversion off, because weeder
  judges bytes and Git for Windows is installed with `core.autocrlf` on.
- Anything reaching for a program rather than a module. `platform-gates.sh` reads
  `std::os::*` uses, so a test that starts a program only unix has is not
  something it can see. The ones this pass found are gated with the reason beside
  them: `bite` and `tests/scan_exec_bound.rs` run their commands through `sh`,
  the pty tests need `openpty`, `tests/scan_offline.rs` takes the network away
  with `unshare` or `sandbox-exec`, and `tests/check_hostile.rs` builds symlinks
  and filenames NTFS refuses.

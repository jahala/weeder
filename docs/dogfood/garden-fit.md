# garden-fit — dogfood notes

Notes from working the garden-fit loop on 2026-09-05. One file per loop; the conductor folds
these into `docs/dogfood.md`.

## tend2

- The loop's `## Tried` did the job it is meant to do. Three entries had already ruled out
  work I would otherwise have started: the umbrella's `scripts/fit/weed.sh` and `garden.lock`
  do not exist, so there was no runner to write against; the landing page is a later loop, so
  the garden footer goes on the README; and the worktree merges guard and hooks, so both
  subcommand arms in `src/main.rs` stay. That is three dead ends I did not walk into, from
  three lines of text.
- The re-pointing entry, needing guard and hooks rather than calibration, is what made the
  `SKILL.md` check possible at all. `SKILL.md` has to name every subcommand the binary prints,
  so it can only be written after the faces exist. A loop that shipped it earlier would have
  had to guess, and the guess would have been wrong by two faces.
- The one friction: a check can name evidence that another loop owns, and nothing on the map
  says so. See the note on `tests/speed.rs` below.

## What is not closed, and who owns it

- **`tests/speed.rs` does not exist.** The check this loop closes asks that `ci.yml` run the
  latency test on a release build; the latency test itself is `weed.tend2.html`'s own check
  and belongs to another loop. `ci.yml` therefore runs `cargo test --release`, the whole
  suite on the release profile, rather than naming a test target that is not written yet.
  When `tests/speed.rs` lands it is inside that run with no change to the workflow, and
  `scripts/check/release.sh` refuses any narrowing of the release run to a subset, so it
  cannot quietly be excluded later.
- **`scripts/check/calibration-bar.sh` does not exist.** `garden.json` names it as the metric
  command, which is all this loop's check asks. The calibration loop writes the script and
  proves it runs, and the same loop adds `metric.latest` to the manifest, the committed
  summary F5 reads. The vendored schema already carries that field as optional, so nothing has
  to move when it arrives.
- **`AGENTS.md`'s layout block does not list the new top-level files**, `garden.json`,
  `SKILL.md`, `LICENSE`, `README.md`, `.brand/`, `.github/`, `npm/`. The conductor owns that
  file and every node branch merges into it, so a worker editing it would conflict with every
  other worker. Left for the conductor.

## Decisions a reader may want the reasoning for

- **`garden.json` declares four faces, not six.** The loop's narrative names `check`, `scan`,
  `guard`, `bite`, `hook` and `rules`. `scan` and `bite` are not in the binary yet, and a
  manifest that claims a face the binary does not answer is exactly the growth weed refuses in
  other people's diffs. `tests/garden_manifest.rs` asserts the declared faces equal what
  `weed --help` prints, so the manifest cannot drift in either direction: when `scan` and
  `bite` land, that test fails until the manifest names them.
- **The mark was drawn twice.** The first version was three stems on a ground rule with one
  pulled clear. The owner's recorded direction is shears cutting a weed, so it was redrawn as
  a pair of open shears around a stem, the judgement one moment before it lands. Both were
  rendered at 16, 20, 28 and 48 pixels and read at each; the shears hold to 20 and lose the
  reading at 16, which is what `.brand/products/weed/identity.md` now says.
- **The contrast numbers are recomputed, not read.** `scripts/check/brand.sh` measures every
  hex in `colors.md` against the background its column names and refuses a row whose recorded
  ratio is more than 0.05 away. A palette table nobody measures is a table where a colour that
  fails its class can be written down as one that passes.
- **`scripts/check/readme.sh` walks the binary's help tree.** Every `weed` command the README
  cites is walked against what the binary prints, subcommands, flags, and the values clap
  says a flag or an argument accepts. `weed rules --format yaml` fails the check. This is the
  only way "cites only commands that exist" means anything once the CLI starts moving.
- **Nothing here deletes.** `scripts/check/clean-install.sh` copies a fresh tree into a temp
  directory and leaves it there, printing the path. The repository's own rule is that deletion
  goes through `trash`, which is not on a Linux runner, so the script does not delete at all.

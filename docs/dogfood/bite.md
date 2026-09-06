# dogfood — bite

## pleach: the phased node commits once, and the premise rested on it

The loop's premise was one sentence: "pleach commits the test phase separately
from the implementation". The whole cheapness of bite came from that. It is not
true of pleach 0.0.1, and finding out took reading two files of its source.

`src/loop/run-work.ts` handles the `{test, phases}` shape by sending each
phase's prompt into the same worker session and running the node's `test`
command after the `red` phase, where a zero exit is a gate failure, and after
the `green` phase, where anything but zero is. That is a real TDD gate, and it
leaves no commit behind: the worker's tree simply moves on to the next prompt.
The commit happens once, later, in `src/loop/run-plan.ts`, where a node that
reaches `done` has its whole staged tree committed onto `node/<id>` with the
subject `pleach: <id> verified (done)` before the verdict is emitted.

So a phased node lands the tests and the implementation in one commit. Anything
downstream that wants the test phase on its own, bite here, a reviewer reading
red-then-green, a bisect that wants to see the test fail, has nothing to work
with.

What would fix it, from bite's side of the fence: commit the tree at the end of
the `red` phase, before the `impl` prompt is sent, and let the close commit sit
on top of it. Two commits per phased node instead of one, and the node's branch
tells the story it was run in. The gate pleach already runs after `red` is
exactly the proof that the commit is worth making, the tests failed, on this
tree, at this moment.

Passing on the smaller thing too: `pleach schema` names the phases `red`, `impl`
and `green`, and only `red` and `green` run the test command. A node whose
phases are `red` then `impl` never runs the command again, so its last state is
never proven green. That is the plan author's business, but a validator warning
would catch it.

## tend2: a check that names two outcomes

The third check here reads "shows at least 90 percent ... or the file records
the kill and `bite` is absent from `weed --help`". One check, two acceptable
worlds, and the evidence script has to decide which world it is in and then hold
the tree to that world's consequence. That worked out well: the script measures
first and enforces second, so it refuses a report that claims the kill while the
binary offers the face, and refuses a hidden face once the measurement clears
the bar.

Worth saying because a check written as a single claim would have forced a
choice between measuring and shipping. The `or` in the check is what let the
honest outcome be a pass rather than a `BLOCKED.md`.

## The kill, and what it costs

The face is built, tested and hidden: `#[command(hide = true)]` on the
subcommand, out of `garden.json`, out of `SKILL.md`, and `docs/bite-2026-09.md`
carries the measurement and the reasons. The two behaviour checks needed the
face to exist and the third needed it not to be offered, and a hidden
subcommand is the only shape that is honest about both, the code is there and
the binary does not offer it.

The alternative was a cargo feature, which would have left the tests either
disabled by default, and therefore theater, or enabling a feature the release
does not build. Neither is a proof.

# weed — voice delta

Inherits the plotplot umbrella voice (calm · precise · literate · a little wit). One signature
line and a terminology table, merged with the umbrella at read time.

**Signature phrase:** *agents produce; weed decides.*

## Terminology

| Use | Not | Why |
|---|---|---|
| finding | violation, error, issue | A finding is something weed found in a change. It is a reading of the diff, not a verdict on the person, and it is not weed failing. |
| block | failure, rejection, denial | The change does not go through. Nothing failed — weed did its job, and exit 2 is the job done. |
| warn | soft error, minor violation | For the human at the pull request, never for the agent. A warn that stops a machine is a block wearing the wrong word. |
| note | info, hint, suggestion | The third level, and the quietest. It records something a reader may want, and asks for nothing. |
| gate | blocker, guardrail, policy engine | A gate is a place a change passes through and may not. It is a shape, not a department. |
| honest | correct, compliant, valid | The question weed asks is whether the diff is what it says it is. Correctness is the test suite's business. |
| growth | change, delta, output | What an agent produced. weed refuses the growth that should not be there and leaves the rest standing. |
| could not run | crashed, failed, broke | Exit 3, and the message says why. weed judged nothing, and says so rather than passing the change through. |
| suppressed | ignored, silenced, muted | A suppression is recorded and still reported under `--strict`. Nothing is ever silently dropped. |
| the diff | the PR, the changeset, the patch | The thing under judgement, named plainly. |

The word **failure** belongs to weed's own runs and to nothing else, and even there
*could not run* is better. Never write that a diff failed, that a check failed, or that a rule
failed; the diff was blocked, and the rule fired.

This table governs weed's prose. It does not govern SARIF, whose levels are `error`, `warning`
and `note` — the standard's words, not weed's, and renaming them in the output would make the
findings unreadable to everything that consumes SARIF. The table `weed check` writes on a
terminal mirrors those level names for the same reason. Around them, in every sentence weed
writes itself, the words above hold.

Findings say what was found, why it matters, and the next action, in that order:
`T1 removed 4 tests from tests/parser.test.ts. The change deletes the proof that parse handles
empty input. Restore the tests, or record why they are gone with a Weed-allow trailer.`

Product name is lowercase always: `weed`. Rule ids are uppercase and bare: `T1`, `S1`, `G1`.
No exclamation marks anywhere; a gate that shouts is a gate nobody reads twice.

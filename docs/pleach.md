# pleach — weeder as the smoke gate

pleach gives every node of a plan its own git worktree, runs the worker there, stages the files that
node was allowed to touch, and then runs the node's `accept.smoke` command in that worktree. The
command's exit code is the gate: zero lands the node, anything else sends it back.

`weeder check --strict` is the whole gate.

```json
"accept": { "smoke": "weeder check --strict" }
```

`examples/pleach/plan.json` is a working plan built that way.

## Why no base argument

With no ref argument weeder judges the index plus the working tree against `HEAD`. That is the staged
files pleach just produced, together with anything the worker left unstaged, which is the change the
node is asking to land. The node's worktree starts at `HEAD`, so everything past it belongs to this
node and a base argument would name the commit weeder already compares against.

The files the worker wrote outside the paths its node was allowed to touch are in that judgement too.
pleach stages the scoped paths, and `git diff` lists nothing else; weeder reads the untracked,
not-ignored files off the working tree itself, so a node that wrote its way around its own scope is
judged on what it wrote rather than on what it staged. `--untracked exclude` puts the narrower view
back for a caller who wants it.

`--base <ref>` is for CI, where a branch has commits of its own and the interesting comparison is
against where it left the trunk. `--staged` is for a pre-commit hook, which judges what the commit
would carry and nothing else.

## Why strict

A worker that cannot make the gate pass can write its own way past it: a `Weeder-allow:` trailer on the
commit message it is preparing, or an inline `weeder-allow` comment beside the line weeder objected to.
Under `--strict` weeder reports those findings at the level their rule carries, so a suppression the
worker wrote for itself still stops the node. `--strict` also refuses to judge a suppression it
cannot read, and leaves with exit 3 rather than guessing what was meant.

Suppressions are written for the human reading the pull request, and `--strict` keeps them in front of that reader.

## Exit codes

| Exit | What it means to pleach |
|---|---|
| 0 | clean, or warnings and notes only; the node lands |
| 2 | at least one block-level finding; the node goes back to its worker |
| 3 | weeder could not run, and the reason is the one line on stderr; the node goes back too |

Exit 3 fails closed on purpose. A gate that could not run must never look like a gate that passed, so
a missing repository, an unreadable ref or a `weeder.toml` weeder cannot parse stops the node exactly as
a finding would.

## What the worker sees

The SARIF log goes to stdout, because pleach reads the command's output from a pipe. A worker reading
its own gate's output gets the rule id, the file, the line and one sentence saying what was found,
why it matters and what to do next. That is enough to fix the change without asking anyone.

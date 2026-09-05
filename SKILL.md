---
name: weed
description: Run weed to judge a diff before a human reads it — deleted or weakened tests, skips, stubs, swallowed errors, secrets, guardrail edits and dependency-direction violations, reported as SARIF and decided by an exit code.
---

# weed

weed reads what an agent produced and refuses growth that should not be there. It
parses nothing itself, spends no tokens, and answers in milliseconds. Three faces
sit on one core: `check` judges a diff and may block, `guard` puts that judgement
inside git, `hook` answers an agent harness. `rules` prints the catalogue.

Exit codes carry the verdict, and they are the same on every face:

| Code | Meaning |
|---|---|
| 0 | clean, or warnings only |
| 2 | at least one block-level finding |
| 3 | weed could not run, and the message says why |

Anything reading weed reads the code first. A run that ends 3 has judged nothing,
so treat it as a refusal to proceed rather than as a pass.

## check — judge a diff

`weed check` compares the index and the working tree against HEAD and writes what
it found. On a terminal it writes a table; on a pipe it writes SARIF 2.1.0. Pass
`--format sarif` or `--format table` to say so outright rather than let the
stream decide.

| Flag | What it does |
|---|---|
| `--base <ref>` | judge the tree against this ref instead of HEAD |
| `--staged` | judge the index alone, the view a pre-commit hook has |
| `--scope <glob>` | the paths the change may touch; every file is still judged, and a file outside the scope is reported for being there. Repeat the flag for more paths |
| `--strict` | report suppressed findings at their own level, and refuse to guess |
| `--format <format>` | `sarif` or `table`, rather than choosing by what stdout is |
| `--config <path>` | read `weed.toml` from here instead of the repository root |
| `--message-file <path>` | the message of the commit being prepared, so its `Weed-allow` trailers are read |

```bash
weed check --base origin/main --strict --format sarif > weed.sarif
weed check --staged
weed check --scope 'src/parser/**' --scope 'tests/parser/**'
```

## guard — the law in git

`weed guard` installs hooks git cannot be talked out of running, by pointing
`core.hooksPath` at them. The six commands under it are the installer and the
hooks themselves; git runs the last three, and you run the first three.

| Command | What it does |
|---|---|
| `install` | write the hooks and point `core.hooksPath` at them |
| `status` | say whether every hook is still live, and name what is not |
| `uninstall` | take the hooks away and put back the hooks path that was there |
| `pre-commit` | judge the index, as git's pre-commit hook |
| `pre-push` | judge what is being pushed, and keep protected branches from being rewritten. git writes the refs on stdin |
| `pre-rebase` | refuse rewriting a protected branch |

`install` takes `--hooks-dir <dir>` to write the hooks somewhere other than
`.githooks`, and `--protect <branch>` to name a branch the hooks refuse to
rewrite. Repeat `--protect` for more than one; leaving it out leaves the hooks
reading `weed.toml` every time git runs them. `pre-push` and `pre-rebase` take
the same `--protect` flag, spelled as the installed bundle spells it.

```bash
weed guard install --protect main --protect release
weed guard status
weed guard uninstall
```

## hook — answer a harness

`weed hook <harness>` reads one hook event as JSON on stdin and answers in the
shape that harness reads. The harness is `claude`, `gemini` or `codex`. Wire it
into the harness's stop or post-edit event, and the turn ends against weed's
judgement rather than the agent's own account of it.

```bash
weed hook claude < event.json
```

## rules — the catalogue

`weed rules` prints every rule and the level it carries. `--format table` is the
default and `--format json` is for a program. Only unambiguous rules block by
default; everything else warns, and a warning is for the human at the pull
request rather than for the agent.

```bash
weed rules
weed rules --format json
```

## Every command

`-h` and `--help` print help on weed and on each of its commands. `-V` and
`--version` print the version.

## Working against weed

- Read the finding, not the exit code alone. Each one says what was found, why it
  matters, and the next action, in that order.
- A block is a statement about the change, not about the run. Restore what was
  weakened, or record a reason weed can read.
- Suppress with a `Weed-allow` trailer on the commit being prepared, and pass it
  with `--message-file`. `--strict` reports every suppression at its own level,
  which is how a reviewer sees what was waved through.
- Never edit a rule, a fixture or a hook to make a finding go away. That is the
  growth weed exists to refuse, and `guard` sees it from inside git.

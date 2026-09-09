---
name: weeder
description: Run weeder to judge a diff or a whole repository before a human reads it, refusing deleted or weakened tests, skips, stubs, swallowed errors, secrets, guardrail edits and dependency-direction violations, reported as SARIF and decided by an exit code.
---

# weeder

weeder reads what an agent produced and refuses growth that should not be there. It
parses nothing itself, spends no tokens, and answers in milliseconds. Four faces
sit on one core: `check` judges a diff and may block, `scan` judges the repository
as it is and never blocks, `guard` puts that judgement inside git, `hook` answers
an agent harness. `rules` prints the catalogue.

Exit codes carry the verdict, and they are the same on every face:

| Code | Meaning |
|---|---|
| 0 | clean, or warnings only |
| 2 | at least one block-level finding |
| 3 | weeder could not run, and the message says why |

Anything reading weeder reads the code first. A run that ends 3 has judged nothing,
so treat it as a refusal to proceed rather than as a pass.

## check: judge a diff

`weeder check` compares the index and the working tree against HEAD and writes what
it found. On a terminal it writes a table; on a pipe it writes SARIF 2.1.0. Pass
`--format sarif` or `--format table` to say so outright rather than let the
stream decide.

A file you wrote and never staged is judged too, as the added file it is. `git
diff` never lists one, so weeder reads them off the working tree itself; the
files your ignore rules hide are not part of the tree and are not read.

| Flag | What it does |
|---|---|
| `--base <ref>` | judge the tree against this ref instead of HEAD |
| `--staged` | judge the index alone, the view a pre-commit hook has |
| `--untracked <include\|exclude>` | whether the files git has never been told about are part of the change. Included by default when the working tree is judged against HEAD, left out with `--staged` and `--base` |
| `--scope <glob>` | the paths the change may touch; every file is still judged, and a file outside the scope is reported for being there. Repeat the flag for more paths |
| `--strict` | report suppressed findings at their own level, and refuse to guess |
| `--format <format>` | `sarif` or `table`, rather than choosing by what stdout is |
| `--config <path>` | read `weeder.toml` from here instead of the repository root |
| `--message-file <path>` | the message of the commit being prepared, so its `Weeder-allow:` trailers are read |

```bash
weeder check --base origin/main --strict --format sarif > weeder.sarif
weeder check --staged
weeder check --scope 'src/parser/**' --scope 'tests/parser/**'
weeder check --untracked exclude
```

## scan: judge the repository as it is

`weeder scan` reads the tree rather than a diff, and reports what the repository
has become: documentation citing a path, command, flag or symbol that no longer
resolves, an export nothing references, a work marker older than the configured
age, a dependency pin the registry left behind. None of that is one change's
fault, so none of it may stop one, every finding is a warning and the run leaves
with 0, or with 3 when weeder could not read the repository at all.

Whether a change is staged, unstaged or committed makes no difference: a scan is
about the state, not about the change.

| Flag | What it does |
|---|---|
| `--rules <ids>` | run these scan rules alone. Repeat the flag, or separate ids with a comma |
| `--format <format>` | `sarif` or `table`, rather than choosing by what stdout is |
| `--config <path>` | read `weeder.toml` from here instead of the repository root |
| `--refresh-snapshot` | ask the registries for the latest release of everything the manifests pin, write `.weeder/registry-snapshot.json`, then scan against it |

```bash
weeder scan --format sarif > hygiene.sarif
weeder scan --rules R1,R3
weeder scan --refresh-snapshot
```

R4 compares a pin against `.weeder/registry-snapshot.json`, a file the repository
commits, and never against a registry: a scan that reached the network would
answer differently on every machine. `--refresh-snapshot` is the one command that
goes and asks, and it asks through `curl`. A refresh that reaches no registry at
all leaves with 3 and keeps the committed snapshot: a file saying the registries
have released nothing would read as every pin being current.

R1 resolves a cited command and its flags against that command's own `--help`,
and only for the commands `[docs] commands` names in `weeder.toml`. With none
named, a cited command is left alone, weeder has no authority to resolve it
against. What a scan may execute is that list and nothing else: the program is
run only with `--help`, the subcommands it walks are the ones the help itself
printed, and no shell and no argument from the document ever reaches a process.
A line like `some-tool; rm -rf build/` in a document is prose weeder reads and
never a command weeder runs.

R1 reads a citation whole and reports it at the level the document earned. A
path keeps the `:line` or `:start-end` written on it, so one that resolves is
still reported when the line is past the end of the file, with the length the
file has. A name a paragraph pins to a file or a directory is asked of that
place first and the tree second, and comes back as a warning naming the place; a
name with nothing beside it is asked of the tree alone and comes back as a note.

## guard: the law in git

`weeder guard` installs hooks git cannot be talked out of running, by pointing
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
reading `weeder.toml` every time git runs them. `pre-push` and `pre-rebase` take
the same `--protect` flag, spelled as the installed bundle spells it.

```bash
weeder guard install --protect main --protect release
weeder guard status
weeder guard uninstall
```

## hook: answer a harness

`weeder hook <harness>` reads one hook event as JSON on stdin and answers in the
shape that harness reads. The harness is `claude`, `gemini` or `codex`. Wire it
into the harness's stop or post-edit event, and the turn ends against weeder's
judgement rather than the agent's own account of it.

```bash
weeder hook claude < event.json
```

## rules: the catalogue

`weeder rules` prints every rule, the level it carries, the face it belongs to and
what it may reach over the network. Every rule but R4 says `none`, and R4 says
`registries under --refresh-snapshot`. `--format table` is the default and
`--format json` is for a program. Only unambiguous rules block by
default; everything else warns, and a warning is for the human at the pull
request rather than for the agent.

```bash
weeder rules
weeder rules --format json
```

## Every command

`-h` and `--help` print help on weeder and on each of its commands. `-V` and
`--version` print the version.

## Working against weeder

- Read the finding, not the exit code alone. Each one says what was found, why it
  matters, and the next action, in that order.
- A block is a statement about the change, not about the run. Restore what was
  weakened, or record a reason weeder can read.
- Allow a finding through on the record, never in silence: a `Weeder-allow: <RULE>
  <reason>` trailer on the commit being prepared, passed in with `--message-file`,
  or a `weeder-allow <RULE>: <reason>` comment on the line itself. Both need a
  reason; one without is a complaint on stderr. `--strict` reports every allowance
  at its own level, which is how a reviewer sees what was waved through.
- Never edit a rule, a fixture or a hook to make a finding go away. That is the
  growth weeder exists to refuse, and `guard` sees it from inside git.

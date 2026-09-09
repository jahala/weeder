# The rules

weeder's whole catalogue: what each rule reports, what it reads to decide, and the
level it carries until a repository says otherwise. `weeder rules` prints the same
rows from the same table in the binary, so this page and the judgement cannot
come apart. Every SARIF result weeder writes links back here by rule id.

`check` judges a diff and may stop a change. `scan` judges the tree and never
does. `bite` runs a test command over two states of a repository, which makes it
the one judgement weeder reaches by running something rather than by reading. A
check rule and a bite rule are `block`, `warn` or `off`; a scan rule is `on` or
`off`. A rule blocks only where what it found admits one reading. The warnings
are for the person at the pull request, not an instruction to the agent that
wrote it.

weeder does not offer the `bite` face today, so the one rule under it reports
nothing: `docs/bite-2026-09.md` holds the measurement that left the face
unshipped, and what would turn it back on.

## What a repository states

`weeder.toml` at the root of the repository, every section optional.

| Section | What it sets | Rules that read it |
|---|---|---|
| `[rules]` | a level per rule id: `block`, `warn` or `off` for a check rule, `on` or `off` for a scan rule | all of them |
| `[scope] allow` | the path globs a change may touch | X2 reports the rest, D1 stops being a warning outside them |
| `[scope] specimens` | the fixture directories no rule reads | all of them, on both faces |
| `[deps] layers` | a layer name to the path globs that belong to it | D2 |
| `[deps] allow` | the `{ from, to }` pairs an import may run between | D2 |
| `[entrypoints] cli` | the modules that are the command line itself, where printing is the product | S3 |
| `[thresholds] todo_age_days` | how old a work marker may get | R3 |
| `[thresholds] dependency_lag` | how far behind a pin may fall | R4 |
| `[guard] protected` | the branches the git hooks refuse to rewrite | `weeder guard` |
| `[guardrails] paths` | the path globs this repository holds at the constitution tier | C3 |

`weeder check --scope <glob>` names the scope for one run and takes precedence
over `[scope] allow`. A run given neither allows every path, so X2 has nothing
to report: weeder will not invent the sentence a change was meant to be held to.

`[scope] specimens` is the one place a repository takes paths away from the
rules, and it may only take away directories under `fixtures/adversarial/`. An
adversarial fixture is written to look dishonest, so weeder reading it as
production code is weeder being right about the wrong file. An entry pointing
anywhere else, a glob that can name a path outside that root, and a rule id in
the list are each refused with exit 3, naming what was written. Every path an
exclusion covers is reported once, at note level, under the id `SPECIMEN`: the
log says which files no rule read, and `check` still leaves with whatever the
files it did read deserve.

## Allowing a finding through

A finding weeder is wrong about, or right about for a reason the change carries anyway,
is allowed through on the record and never in silence. There are two ways to write one,
and both need a reason:

| Where | How it is written |
|---|---|
| the commit being prepared | a `Weeder-allow: <RULE> <reason>` trailer, handed to weeder with `--message-file` |
| the line itself | a `weeder-allow <RULE>: <reason>` comment beside it |

An allowance turns the finding into a `note` that stops nobody and stays in the log, carrying
the reason as a SARIF suppression. `weeder check --strict` gives it back the level its rule
carries and reports it anyway: an agent that wrote itself an allowance is still stopped, and
the reviewer sees both the finding and what was said about it. A marker with no reason is a
line weeder cannot read: it says so on stderr, and under `--strict` it refuses to judge the
file at all.

A trailer reaches weeder through `--message-file` or on the commits of a `--base` range,
never from git's own `COMMIT_EDITMSG`, which at pre-commit time still holds the previous
commit's message. An allowance must not outlive the change it was written for.

<a id="T1"></a>
## T1: A test was deleted

`check` · blocks by default

A test file was removed, or a test case disappeared from a changed test file, counted from the file's classification and its test shape at HEAD against the working tree. A case the same change put into another test file moved rather than went, and is left out of the count and of what the finding claims.

<a id="T2"></a>
## T2: Assertions were dropped from a changed test file

`check` · blocks by default

The assertion count of a changed test file fell between HEAD and the working tree, and no allowance carries a reason for it. An assertion the same change makes in another test file moved rather than went, and is left out of the count and of what the finding claims.

<a id="T3"></a>
## T3: A skip or focus marker was added

`check` · blocks by default

An added line in a test file carries a skip, focus or todo marker of the file's test framework, matched as syntax rather than as a substring of a string or a comment.

<a id="T4"></a>
## T4: A tolerance or a timeout was widened

`check` · warns by default

A removed and an added line in one hunk hold the same approximate assertion or timeout with a larger number, so the test now accepts what it used to refuse.

<a id="T5"></a>
## T5: Expected values were regenerated

`check` · warns by default

Snapshot, golden or fixture files changed in a diff that also changes production code, so the expectation moved to meet the code.

<a id="T6"></a>
## T6: An error assertion was weakened

`check` · warns by default

A removed and an added line turned an assertion on a specific error message or type into one that accepts any error.

<a id="T7"></a>
## T7: A rename took a test out of the runner

`check` · blocks by default

A test file or case was renamed out of the naming convention its runner collects, so the test still looks present and no longer runs.

<a id="T8"></a>
## T8: A configuration line took a test out of the run

`check` · blocks by default

An added or changed line of the runner's settings stops an existing test file from being collected, read from the settings file's kind and name rather than from any product. A line that names the file, or a pattern matching one, blocks; a line that narrows what the runner reads leaves the hidden set to be worked out, and warns. A setting whose value is built rather than written is reported unreadable instead of guessed at.

The settings a repository writes are found the way its manifest is: by what the classifier makes of the file and by the name its ecosystem gives it. What a key means is read from the words it is spelled with — `ignore`, `exclude`, `deselect` on one side, `path`, `files`, `match`, `include` on the other — and a key counts as being about the run only where the key, the table it sits under, or the file's own name says so. A gate written at the top of a suite, on a condition an ordinary run never sets, is the same exclusion written inside the file it hides. `weeder check --strict` refuses the run, at exit 3, where a setting weeder cannot read is among the lines the change wrote.

<a id="M1"></a>
## M1: A test mocks the unit under change

`check` · warns by default

A test in the diff mocks a module whose production file is also in the diff, so the test no longer exercises the change it covers.

<a id="S1"></a>
## S1: A stub or a TODO reached production code

`check` · blocks by default

An added line in a production file carries a stub marker, an unimplemented body, or a function body that only returns nothing. A body left unwritten in a base type states a contract rather than a stub, so the finding is dropped where production code somewhere in the tree is built on that type and writes the method, and reported at warn where nothing writes it and an entry file states the type, because the implementation may be in another repository.

<a id="S2"></a>
## S2: An error was swallowed

`check` · warns by default

An added handler in production code discards a failure without logging it, wrapping it, or passing it on.

<a id="S3"></a>
## S3: A debug leftover reached production code

`check` · warns by default

An added line in a production file prints or breaks for debugging, outside the entry points the config names as command-line faces.

<a id="D1"></a>
## D1: A dependency manifest changed

`check` · warns by default

A package manifest or its lockfile changed; the change blocks when a scope excludes the manifest.

<a id="D2"></a>
## D2: An import crossed a forbidden boundary

`check` · blocks by default

A changed file imports across a direction that `[deps] allow` does not permit, by the layers `[deps] layers` names.

<a id="X1"></a>
## X1: A secret-looking string was added

`check` · blocks by default

An added line carries a known credential prefix, or a high-entropy literal assigned to a name that reads like a key. A credential a vendor published in its own documentation is named as an example and reported as a note.

<a id="X2"></a>
## X2: A file outside the scope was touched

`check` · blocks by default

A changed file matches no scope glob the run allows, and the finding names what depends on the definitions it changed.

<a id="C1"></a>
## C1: A guardrail file was edited

`check` · blocks by default

A change touched a harness settings file, a git hook, `weeder.toml`, or the hard-limits section of `AGENTS.md` or `CLAUDE.md`.

<a id="C2"></a>
## C2: An ignore file was broadened over source or tests

`check` · warns by default

An added ignore pattern matches source or test paths of the repository's languages, hiding them from review and from tooling.

<a id="C3"></a>
## C3: A workflow was changed

`check` · warns by default

A file under `.github/workflows/` was added, edited or taken away; the change blocks where `[guardrails] paths` names the path.

<a id="G1"></a>
## G1: A conflict marker was committed

`check` · blocks by default

An added line is a merge conflict marker, so the file carries both sides of a merge nobody finished.

<a id="G2"></a>
## G2: A large or a binary file was added

`check` · warns by default

An added file is larger than one mebibyte, or holds binary content, and was not tracked before.

One mebibyte is also where weeder stops reading a file as code. Above it a file is
weighed and its lines are read, and nothing is asked of the parser: an outline of
a blob nobody will open is the most expensive question a run can ask and it
answers none anybody had. Every rule that judges a line still judges every line
of a file that size, so this is a cost weeder declines to pay rather than a place
to hide a change in.

<a id="B1"></a>
## B1: A test passed without the change it covers

`bite` · blocks by default

The test command passed with the test commit alone applied to the base, so the cases that commit added were green before the implementation existed.

The cases are named from the test shape of each side of the test commit, so what a finding points at is the case that commit added, at the line it was written on. A file whose language weeder reads no tests in is reported by the file instead.

<a id="R1"></a>
## R1: The docs cite something that no longer exists

`scan` · on by default

A path, command, flag or symbol cited in the repository's markdown does not resolve against the tree.

A citation carries the line it names. A path that resolves is still wrong when the line is past the end of the file, and the finding says how long the file is. A bare name is answered by whatever the paragraph pinned it to: a path or a directory cited on the same line, or the last one that resolved under the same heading. That place is asked first, the rest of the tree second, and what neither answered is a warning naming the place, because a reader can go there and see. A name with nothing beside it is asked of the tree alone and reported as a note: prose is full of words that are nobody's symbol, and the loud half of this rule is the half a document vouched for.

<a id="R2"></a>
## R2: A public symbol has no references

`scan` · on by default

An exported definition is referenced nowhere in the repository, and it is not one of the entry points its language exempts.

<a id="R3"></a>
## R3: A TODO is older than the configured age

`scan` · on by default

A line carrying `TODO`, `FIXME` or `XXX` was last touched further back than `[thresholds] todo_age_days` allows.

<a id="R4"></a>
## R4: A dependency pin lags the registry

`scan` · on by default

A manifest pin is further behind the latest release than `[thresholds] dependency_lag` allows, measured against the committed registry snapshot. `weeder scan --refresh-snapshot` is the one path that reaches the registries, and it asks curl to do the reaching.

<a id="R5"></a>
## R5: A test file the configuration never collects

`scan` · on by default

A test file the tree holds is one the runner's settings never collect, named once per file. It is the tree-state twin of T8, so a repository that arrived with the exclusion already written is told as plainly as one that adds it.

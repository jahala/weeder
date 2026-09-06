# The rules

weed's whole catalogue: what each rule reports, what it reads to decide, and the
level it carries until a repository says otherwise. `weed rules` prints the same
rows from the same table in the binary, so this page and the judgement cannot
come apart. Every SARIF result weed writes links back here by rule id.

`check` judges a diff and may stop a change. `scan` judges the tree and never
does. A check rule is `block`, `warn` or `off`; a scan rule is `on` or `off`. A
rule blocks only where what it found admits one reading. The warnings are for
the person at the pull request, not an instruction to the agent that wrote it.

## What a repository states

`weed.toml` at the root of the repository, every section optional.

| Section | What it sets | Rules that read it |
|---|---|---|
| `[rules]` | a level per rule id: `block`, `warn` or `off` for a check rule, `on` or `off` for a scan rule | all of them |
| `[scope] allow` | the path globs a change may touch | X2 reports the rest, D1 stops being a warning outside them |
| `[deps] layers` | a layer name to the path globs that belong to it | D2 |
| `[deps] allow` | the `{ from, to }` pairs an import may run between | D2 |
| `[entrypoints] cli` | the modules that are the command line itself, where printing is the product | S3 |
| `[thresholds] todo_age_days` | how old a work marker may get | R3 |
| `[thresholds] dependency_lag` | how far behind a pin may fall | R4 |
| `[guard] protected` | the branches the git hooks refuse to rewrite | `weed guard` |
| `[guardrails] paths` | the path globs this repository holds at the constitution tier | C3 |

`weed check --scope <glob>` names the scope for one run and takes precedence
over `[scope] allow`. A run given neither allows every path, so X2 has nothing
to report: weed will not invent the sentence a change was meant to be held to.

<a id="T1"></a>
## T1: A test was deleted

`check` · blocks by default

A test file was removed, or a test case disappeared from a changed test file, counted from the file's classification and its test shape at HEAD against the working tree.

<a id="T2"></a>
## T2: Assertions were dropped from a changed test file

`check` · blocks by default

The assertion count of a changed test file fell between HEAD and the working tree, and no allowance carries a reason for it.

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

<a id="M1"></a>
## M1: A test mocks the unit under change

`check` · warns by default

A test in the diff mocks a module whose production file is also in the diff, so the test no longer exercises the change it covers.

<a id="S1"></a>
## S1: A stub or a TODO reached production code

`check` · blocks by default

An added line in a production file carries a stub marker, an unimplemented body, or a function body that only returns nothing.

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

An added line carries a known credential prefix, or a high-entropy literal assigned to a name that reads like a key.

<a id="X2"></a>
## X2: A file outside the scope was touched

`check` · blocks by default

A changed file matches no scope glob the run allows, and the finding names what depends on the definitions it changed.

<a id="C1"></a>
## C1: A guardrail file was edited

`check` · blocks by default

A change touched a harness settings file, a git hook, `weed.toml`, or the hard-limits section of `AGENTS.md` or `CLAUDE.md`.

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

<a id="R1"></a>
## R1: The docs cite something that no longer exists

`scan` · on by default

A path, command, flag or symbol cited in the repository's markdown does not resolve against the tree.

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

A manifest pin is further behind the latest release than `[thresholds] dependency_lag` allows, measured against the committed registry snapshot. `weed scan --refresh-snapshot` is the one path that reaches the registries, and it asks curl to do the reaching.

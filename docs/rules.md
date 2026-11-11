# The rules

weeder's whole catalogue: what each rule reports, what it reads to decide, and the
level it carries until a repository says otherwise. `weeder rules` prints the same
rows from the same table in the binary, under a header naming its columns: the
id, the level, the face, what the rule may reach over the network, and what it
reports. This page and the judgement cannot come apart. Every SARIF result weeder writes links back here by rule id.

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

For `check` and the hooks it is read from the base of the judged range, never
from the change: HEAD for the working tree and for `--staged`, the base commit for
`--base` and for pre-push. A change to `weeder.toml` is judged under the law it
is changing, C1 included, and takes effect once it lands. A base with no
`weeder.toml` is judged at the levels below. `check --config <path>` names
another law for one run.

A release that renames or removes a key leaves a base whose `weeder.toml` the new
binary cannot read. Over such a base weeder judges one change only: one that
rewrites `weeder.toml` and touches no other path, in the diff and in every
commit of the range. It is judged at the levels below, as if the base carried no
law, and the run says so in one note under the id `LAW`. The change still writes
no law for itself, so C1 blocks it until a person signs it with a
`Weeder-allow: C1` trailer, or at the pull request where CI runs `--strict`. Any
other change over that base exits 3, and the message says to land the migration
first, on its own. `scan` judges the tree it is given, and reads the
`weeder.toml` that tree holds.

| Section | What it sets | Rules that read it |
|---|---|---|
| `[rules]` | a level per rule id: `block`, `warn` or `off` for a check rule, `on` or `off` for a scan rule | all of them |
| `[scope] allow` | the path globs a change may touch | X2 reports the rest, D1 stops being a warning outside them |
| `[scope] exclude` | the path patterns no rule reads, gitignore-style from the root | all of them, on both faces |
| `[deps] layers` | a layer name to the path globs that belong to it | D2 |
| `[deps] allow` | the `{ from, to }` pairs an import may run between | D2 |
| `[entrypoints] cli` | the modules that are the command line itself, where printing is the product and what they export is the command line's to call | S3, S4 |
| `[entrypoints] loaded` | the path globs of files another program loads by the path it is given, whose exports are that program's to call | R2, S4 |
| `[thresholds] todo_age_days` | how old a work marker may get | R3 |
| `[thresholds] dependency_lag` | how far behind a pin may fall | R4 |
| `[docs] commands` | the programs whose own `--help` a cited command and its flags are resolved against | R1 |
| `[docs] external` | the names the docs cite on purpose that live outside the tree: file names, paths, globs, variables | R1 |
| `[guard] protected` | the branches the git hooks refuse to rewrite | `weeder guard` |
| `[guardrails] paths` | the path globs this repository holds at the constitution tier | C3 |
| `[tests] assertions` | the names of the repository's own test helpers, each matched whole, whose calls are assertions in every language | T2 |
| `[guardrails] sections` | the headings of `AGENTS.md` and `CLAUDE.md` whose sections state this repository's law, beside the ones saying hard limits or non-negotiables, or opening with never, matched as words in any case | C1 |
| `[[overrides]] paths` | the path patterns an override applies to, gitignore-style from the root | the rules it names |
| `[[overrides]] rules` | a level per rule id under those paths, as `[rules]` spells them | the rules it names |

`weeder check --scope <glob>` names the scope for one run and takes precedence
over `[scope] allow`. A run given neither allows every path, so X2 has nothing
to report: weeder will not invent the sentence a change was meant to be held to.

`[scope] exclude` takes paths away from every rule. A repository keeps files
that look dishonest on purpose, fixtures, recorded runs of other projects, a
frozen old tree, and weeder reading them as production code is weeder being
right about the wrong file. The patterns read like `.gitignore` lines anchored
at the root: `test/fixtures/` is a directory and everything under it,
`docs/dogfood/**` the same by its contents, `**/*.snap` a kind of file at any
depth.

```toml
[scope]
exclude = ["test/fixtures/", "docs/dogfood/**", "**/*.snap"]
```

`[[overrides]]` sets rule levels for some paths and keeps the rule everywhere
else. A level written there caps every finding of that rule under those paths,
the way one under `[rules]` caps it everywhere, and an override can turn on
under its paths a rule `[rules]` turns off. Where several overrides cover a file
and name the same rule, the most specific pattern wins: the one with more of its
path written out, `docs/keep/` over `docs/**`. Two equally specific patterns
that can cover one file and set one rule two ways are refused, because either
pick would be a level nobody wrote for that file.

```toml
[[overrides]]
paths = ["docs/research/**", "docs/dogfood/**"]
rules = { R1 = "off", R2 = "off" }
```

A gate that can be told not to read its own law is no gate, so some patterns
are refused in both keys, each with exit 3 naming the pattern and the way
forward: an absolute path, a pattern that steps out with `..`, one that matches
everything (`*`, `**`, `/`), and one that names a guardrail file, `weeder.toml`,
`AGENTS.md`, `CLAUDE.md`, the hooks directory, `.github/workflows/`, the agent
harness settings, or a path `[guardrails] paths` names. A guardrail file is
never excluded, whatever pattern covers it, so a pattern that reaches one only
through a wildcard, `**/testdata/` against `.githooks/testdata/`, leaves it
judged.

Three more things keep an exclusion honest. The law comes from the base, so a
pattern a change adds applies once that change has landed. `weeder.toml` is a
guardrail, so C1 blocks the change that writes one until a person allows it,
and CI under `--strict` honours no allowance. A file moved into an excluded path
is judged as the deletion of the path it left, so moving a test out of sight
still blocks on T1.

R1 still counts an excluded path as present, so a doc that cites a fixture is
not a broken reference. Each pattern that set a file aside is reported once, at note level, under the id
`EXCLUDED`, with the count of files it took. `check` and `scan` both say it, and
the SARIF log carries it, so a reviewer sees what was not judged, and `check`
still leaves with whatever the files it did read deserve.

`check` also says, once per run, which changed files it did not read: code in a
language weeder has no rules for, a makefile, Java, Ruby. The note carries the
id `UNREAD`, counts the files by extension, and stops nothing. The rules judge
TypeScript, JavaScript, Python, Rust, Go and bash, and a change made only of
other code would otherwise end in a clean count over lines no rule read.

Bash is the newest of the six, and not every rule reads it yet. `.sh`, `.bash`
and `.bats` files are bash. A `.bats` file, and a script named `*.spec.sh`,
`*_test.sh` or `test_*.sh`, is a test, as is any file under a `tests/`
directory; every other bash file is production code. A rule that reads bash
says so in its section below, and a rule that does not say so reads none of it.

### How a finding's level is settled

Most findings carry their rule's level: the one listed below, or the one
`[rules]` writes for the rule. A few carry a level of their own, because the
detector knows something about that one finding the config cannot. D1 blocks on
a manifest outside the run's scope. C3 blocks on a workflow `[guardrails] paths`
names. T1, T2 and T8 warn where the fall or the hidden set cannot be proved. S1
warns on a base method an entry file hands out. X1 reports a published example
key as a note, and R1 does the same for a bare name nothing pins to a place.

A level the repository writes caps those findings too. `D1 = "warn"` blocks
nothing, and a finding that asked for a quieter level than the one written keeps
its own. `off` silences the rule. A rule the repository leaves out of `[rules]`
reports each finding at the level its detector asked for.

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

### Where a hook reads one

The two ways differ in whose hand wrote them, and the git hooks are where that
matters. `weeder guard commit-msg` judges the index with the message's trailers
honoured, and is the deciding stage for any block-level finding a trailer may
allow, and it prints the findings. `weeder guard pre-commit` runs before the
message exists, so it honours nothing: it prints one line naming what blocks and,
for each rule, the exact trailer that would allow it, `Weeder-allow: <RULE> <reason>`,
and says commit-msg is the stage that reads it. The table comes once, from
commit-msg, because the two stages judge one index. `weeder guard pre-push` honours a trailer again over the commits
being pushed, because a commit carries the message it was allowed by.

The deferral holds only while the stage is there to run. `weeder guard pre-commit`
reads the directory git runs hooks from and takes any executable file that
invokes `weeder guard commit-msg` as the stage, whoever installed it; with none
there it refuses on its own rather than passing the verdict to nobody, and prints
the table itself.

A `weeder-allow` marker on a line is never honoured by a hook, at any stage. The
line is the agent's to write and the commit message is the person's, and the
allowance a gate honours has to be the person's act.

<a id="T1"></a>
## T1: A test was deleted

`check` · blocks by default

A test file was removed, or a test case disappeared from a changed test file, counted from the file's classification and its test shape at HEAD against the working tree. A case generated from a table counts once for each row of every table around it, a table nested in a table or spread from another included. A case the same change put into another test file moved rather than went, and is left out of the count and of what the finding claims. A fall whose cases now come from a new table the file does not write down is reported as a warning rather than a block, since that table may hold every case that left and only a person can say whether it does.

<a id="T2"></a>
## T2: Assertions were dropped from a changed test file

`check` · blocks by default

The assertion count of a changed test file fell between HEAD and the working tree, and no allowance carries a reason for it. An assertion inside a parametrized case or a loop counts once for each case the file writes down, so a table that loses a row loses a claim. Loose assertions on one value that the change replaces with an exact one on the same value, an equality with the whole of it, are covered by that one and not counted as dropped. A fall that leaves claims under a new loop whose cases the file does not write down is reported as a warning rather than a block, since that loop may run every assertion that left and only a person can say whether it does. An assertion the same change makes in another test file moved rather than went, and is left out of the count and of what the finding claims.

T2 reads bash. A bats suite or a shell spec asserts with the shell's own tests, a command that opens with `[`, `[[` or `test`, and with the `assert_*` and `refute_*` helpers bats suites load. A test an `if`, an `elif`, a `while` or an `until` asks is a condition, and claims nothing. A `for name in a b c` loop runs its claims once for each word it writes out, and a loop over `"$@"`, a glob or a substitution is one whose cases the file does not write down. A bats case has no name weeder can read yet, so a claim is followed between files by its line alone.

A suite that asserts through a helper of its own names it under `[tests] assertions` in `weeder.toml`, and T2 counts a call to it as an assertion in every language weeder reads: `t(...)` in TypeScript, Python, Rust and Go, and a `t` command in bash. The name is matched whole, so naming `t` leaves `test(...)` and `t.Run(...)` what they were.

```toml
[tests]
assertions = ["t", "assert_rendered"]
```

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

Snapshot, golden or fixture files that were already there changed in a diff that also changes production code, so the expectation moved to meet the code. A file the same change adds, and a test double the tests run, a file git keeps executable or one that opens with a shebang, are not expectations.

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

An added or changed line of the runner's settings stops an existing test file under the settings file's own directory from being collected, read from the settings file's kind and name rather than from any product. A setup, teardown or coverage setting chooses no test file. A line that names the file, or a pattern matching one, blocks; a line that narrows what the runner reads leaves the hidden set to be worked out, and warns. A setting whose value is built rather than written is reported unreadable instead of guessed at.

The settings a repository writes are found the way its manifest is: by what the classifier makes of the file and by the name its ecosystem gives it. What a key means is read from the words it is spelled with — `ignore`, `exclude`, `deselect` on one side, `path`, `files`, `match`, `include` on the other — and a key counts as being about the run only where the key, the table it sits under, or the file's own name says so. A gate written at the top of a suite, on a condition an ordinary run never sets, is the same exclusion written inside the file it hides. `weeder check --strict` refuses the run, at exit 3, where a setting weeder cannot read is among the lines the change wrote.

<a id="T10"></a>
## T10: An added test restates a constant of the code

`check` · warns by default

A test case the change added checks nothing but a value the code defines against a literal: every assertion compares one name, read and not called, with a literal, and the name is the code's rather than the case's, imported by the file, defined by a production file outside the tests it carries, or reached without an import through Go's package scope or a glob import. A name the case binds, takes as a parameter or its own file defines is the test's own expectation and is left alone.

`expect(SERVICE).toBe("build.example.app")` passes whatever the code does with `SERVICE`, because changing the value changes the case with it. A case that hands the constant to the code and checks what comes back, `expect(endpoint(SERVICE)).toBe("https://build.example.app/")`, can fail, and T10 leaves it alone. So does an expected value imported from the code and compared with what the code returns, `expect(status()).toBe(Status.Ready)`: the import only names the answer.

<a id="T11"></a>
## T11: An added test asserts only on its own double

`check` · warns by default

A test case the change added asserts on nothing but a double it built itself: every assertion reads a name the case bound to a double, made by a call or a type whose name says it makes one, or a value the case read off that double, and the case hands the double to nothing else, so no assertion reads what the code under test did. A double handed to the unit, with an assertion on the unit's output or on what the unit did to the double, is left alone.

A double is read through the same words M1 reads one by, `mock`, `stub`, `fake`, `spy`, `patch` and `double`, and through the script frameworks' mock function, a member called `fn` and called. A case that tells `vi.fn()` to answer `true` and then checks that it answered `true` has tested the mock. Configuring the double, `save.mockReturnValue(true)` or `clock.set(5)`, is not handing it on; passing it as an argument, or as a value in an object, is.

<a id="T12"></a>
## T12: An added test only checks that its input came back

`check` · warns by default

A test case the change added checks nothing but that a string it passed in comes back out: every assertion holds what it reads to a string literal, outside the value it reads, that the case already wrote on an earlier line that is not a check, a prop, an argument or a fake's message. A literal of nothing but punctuation is no message and is passed over. A check that holds the answer to a literal of its own, the input with what the code adds to it, is left alone.

A case that builds `new Error("Workspace not found")`, hands it to the code and checks that the answer contains `Workspace not found` stays green as long as the string gets through, whatever the code does around it. A case that stores a value and reads it back has the same shape and can be honest, which is one reason T12 warns rather than blocks.

<a id="M1"></a>
## M1: A test mocks the unit it should exercise

`check` · warns by default

A test in the diff mocks a module whose production file is also in the diff, so the test no longer exercises the change it covers. A double the change sets up of what the test is about is reported too, whether the change edits that unit or not: a module double of the file the test file is named for, or a double of what the title of the added case it serves, or of a suite around that case, names.

<a id="S1"></a>
## S1: A stub or a TODO reached production code

`check` · blocks by default

An added line in a production file carries a stub marker, an unimplemented body, or a function body that only returns nothing. A body left unwritten in a base type states a contract rather than a stub, so the finding is dropped where production code somewhere in the tree is built on that type and writes the method, and reported at warn where nothing writes it and an entry file states the type, because the implementation may be in another repository.

S1 reads bash. A stub there is a work marker in code or in a comment, an `exit` or a `return` on a line whose message says the work is not implemented, or a function whose only command is `:`, the shell's command for doing nothing. A marker inside a string or a heredoc is what the script prints, and is left alone.

<a id="S2"></a>
## S2: An error was swallowed

`check` · warns by default

An added handler in production code discards a failure without logging it, wrapping it, passing it on, or saying in a comment why it is right to drop it.

S2 reads bash. A command that throws its own error output away (`2>/dev/null`, `&>/dev/null`, `>/dev/null 2>&1`) and is then answered with `|| true` or `|| :` is a failure turned into success with nothing left of it. A `|| true` whose error still prints only keeps the script going, and S2 leaves it alone. A comment on the line, or above the command it ends, is the reason S2 asks for. A backslash that carries the command onto the next line keeps it one command.

<a id="S3"></a>
## S3: A debug leftover reached production code

`check` · warns by default

An added line in a production file prints or breaks for debugging, outside the entry points the config names as command-line faces.

S3 reads bash, where printing is most of what a script is for. So a print is a leftover only when it says it is one: an `echo` or a `printf` whose first word is the label `DEBUG`, or `debug:` or `[debug]`. The other shape is the shell told to trace itself, `set -x`, `set -v`, `set -o xtrace` or a run of flags holding either letter, such as `set -euxo pipefail`. `set +x`, a sentence that happens to begin with the word, and anything inside a string, a comment or a heredoc are output, and are left alone. A test file, a file named `main.*`, a `main` function and a path `[entrypoints] cli` names are exempt, as in every other language.

<a id="S4"></a>
## S4: Production code was added that only its tests call

`check` · warns by default

A production file gains an exported definition, or exports one it already held, and every reference to it outside its own file is in a test, with at least one there. A reference is a call found anywhere in the tree, or the name written in the code of a file the change touched. The entry points their language exempts, the modules `[entrypoints] cli` names and the files `[entrypoints] loaded` names are left alone, and so is an export nothing references, which is R2's.

<a id="D1"></a>
## D1: A dependency manifest changed

`check` · warns by default

A package manifest or its lockfile changed in more than the package's own version; the change blocks when a scope excludes the manifest.

<a id="D2"></a>
## D2: An import crossed a forbidden boundary

`check` · blocks by default

A changed file imports across a direction that `[deps] allow` does not permit, by the layers `[deps] layers` names.

<a id="X1"></a>
## X1: A secret-looking string was added

`check` · blocks by default

An added line carries a known credential prefix, or a high-entropy literal assigned to a name that reads like a key, in source or in an env, YAML, TOML or INI file, where the literal is one run of characters rather than a sentence and reads nothing from the environment. A credential a vendor published in its own documentation is named as an example and reported as a note.

<a id="X2"></a>
## X2: A file outside the scope was touched

`check` · blocks by default

A changed file matches no scope glob the run allows, and the finding names what depends on the definitions it changed.

<a id="C1"></a>
## C1: A guardrail file was edited

`check` · blocks by default

A change touched a harness settings file, a git hook, `weeder.toml`, the garden's `garden.lock` or `garden.json`, or a section of `AGENTS.md` or `CLAUDE.md` whose heading says hard limits or non-negotiables, or opens with never, or names one of `[guardrails] sections`. A release that moves only the product's own version in `garden.json`, its `version` and the `v<version>` segment of each asset URL, is not an edit.

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

A path, command, flag or symbol cited in the repository's markdown does not resolve against the tree. A link is judged by its destination alone, read from the directory of the document that writes it.

A citation carries the line it names. A path that resolves is still wrong when the line is past the end of the file, and the finding says how long the file is. A bare name is answered by whatever the paragraph pinned it to: a path or a directory cited on the same line, or the last one that resolved under the same heading. That place is asked first, the rest of the tree second, and what neither answered is a warning naming the place, because a reader can go there and see. A name with nothing beside it is asked of the tree alone and reported as a note: prose is full of words that are nobody's symbol, and the loud half of this rule is the half a document vouched for.

A path the root does not hold is asked of the tails of every tracked path, on whole components, because docs are often written from inside a source root: `rules/scan/r1.rs` resolves to `src/core/rules/scan/r1.rs` when that is the only path ending that way. A tail several paths end in is reported with the paths it could mean. A brace pattern such as `src/core/rules/scan/{r1,r2}.rs` resolves when every place it spells does, and is reported for the ones that do not.

A symbol resolves when the code declares it, uses it or quotes it in a string,
or when a file of data the repository carries holds it. A name in a string is
one the program still carries, such as a case label or a variable in an
allowlist. A name that only a comment still mentions is reported.

Some names are cited on purpose and never live in the tree: a file a program writes at run time, a variable another program reads. `[docs] external` lists them, as file names, paths, globs or variable names, and R1 reports none of them. A name the list does not cover is judged as before. A path `[scope] specimens` sets aside is still in the repository, so R1 counts it as there without reading it.

<a id="R2"></a>
## R2: A public symbol has no references

`scan` · on by default

An exported definition is referenced nowhere in the repository, and it is not one of the entry points its language exempts or in a file `[entrypoints] loaded` names.

Some files are loaded by another program from the path it is given: a config
file a tool reads with a dynamic `import(path)`, a spec a runner is pointed at.
Nothing in the repository imports what they export, by design. List them under
`[entrypoints] loaded`, as path globs from the root, and R2 treats each as an
entry point:

```toml
[entrypoints]
loaded = ["*.config.ts", "loops/*/loop.ts"]
```

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

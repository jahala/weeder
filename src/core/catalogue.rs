use crate::core::finding::Level;

/// The face a rule belongs to: `check` judges a diff, `scan` judges the tree,
/// `bite` runs a test command over two states of one and judges what it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Check,
    Scan,
    Bite,
}

/// What a rule may reach over a network, and under which flag. weeder is a judge:
/// it reads a tree and writes a verdict, and the one question it cannot answer
/// from what the repository holds is what the registries have released since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// The rule reads what the repository already holds.
    None,
    /// What the rule may reach, and the flag without which it does not.
    Reaches {
        what: &'static str,
        under: &'static str,
    },
}

impl Network {
    /// The one line `weeder rules` prints in its network column.
    #[must_use]
    pub fn spelled(self) -> String {
        match self {
            Network::None => "none".to_string(),
            Network::Reaches { what, under } => format!("{what} under {under}"),
        }
    }
}

/// One rule as the catalogue knows it. This is the single source for the ids
/// `weeder rules` prints, the defaults `Config::default()` ships, and the rules
/// array in a SARIF tool component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub id: &'static str,
    pub face: Face,
    pub default_level: Level,
    /// What was found, as one line.
    pub short_description: &'static str,
    /// What the rule reads to decide, so a consumer never guesses.
    pub full_description: &'static str,
    /// What the rule may reach over a network. Every rule but one says none,
    /// and a rule that starts reaching for something says so here first.
    pub network: Network,
}

const CATALOGUE: &[Rule] = &[
    Rule {
        id: "T1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A test was deleted",
        full_description: "A test file was removed, or a test case disappeared from a changed test file, counted from the file's classification and its test shape at HEAD against the working tree. A case generated from a table counts once for each row of every table around it, a table nested in a table or spread from another included. A case the same change put into another test file moved rather than went, and is left out of the count and of what the finding claims. A case renamed and rewritten as it moved is followed by the production function it calls: where that function is defined on both sides of the change, in files the change touched, and the change adds cases to another test file that call it and make at least as many assertions as the case did, counted as T2 counts them, the case moved. Each added assertion pays once, a case that asserted nothing is followed by its name or its body alone, and a function defined in a file the change left alone follows nothing, so tests gone with no code moved still block. A fall whose cases now come from a new table the file does not write down is reported as a warning rather than a block, since that table may hold every case that left and only a person can say whether it does.",
        network: Network::None,
    },
    Rule {
        id: "T2",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "Assertions were dropped from a changed test file",
        full_description: "The assertion count of a changed test file fell between HEAD and the working tree, and no allowance carries a reason for it. An assertion inside a parametrized case or a loop counts once for each case the file writes down, so a table that loses a row loses a claim. Loose assertions on one value that the change replaces with an exact one on the same value, an equality with the whole of it, are covered by that one and not counted as dropped. A fall that leaves claims under a new loop whose cases the file does not write down is reported as a warning rather than a block, since that loop may run every assertion that left and only a person can say whether it does. An assertion the same change makes in another test file moved rather than went, and is left out of the count and of what the finding claims. An assertion whose case was renamed and rewritten on its way is followed the way T1 follows the case, by the production function the case calls: each assertion added in another test file to a case that calls the same function pays for one that left, and where fewer arrived than left the finding reports the shortfall and names the file the rest went to.",
        network: Network::None,
    },
    Rule {
        id: "T3",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A skip or focus marker was added",
        full_description: "An added line in a test file carries a skip, focus or todo marker of the file's test framework, matched as syntax rather than as a substring of a string or a comment.",
        network: Network::None,
    },
    Rule {
        id: "T4",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A tolerance or a timeout was widened",
        full_description: "A removed and an added line in one hunk hold the same approximate assertion or timeout with a larger number, so the test now accepts what it used to refuse.",
        network: Network::None,
    },
    Rule {
        id: "T5",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "Expected values were regenerated",
        full_description: "Snapshot, golden or fixture files that were already there changed in a diff that also changes production code, so the expectation moved to meet the code. A file the same change adds, and a test double the tests run, a file git keeps executable or one that opens with a shebang, are not expectations.",
        network: Network::None,
    },
    Rule {
        id: "T6",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An error assertion was weakened",
        full_description: "A removed and an added line turned an assertion on a specific error message or type into one that accepts any error.",
        network: Network::None,
    },
    Rule {
        id: "T7",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A rename took a test out of the runner",
        full_description: "A test file or case was renamed out of the naming convention its runner collects, so the test still looks present and no longer runs.",
        network: Network::None,
    },
    Rule {
        id: "T8",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A configuration line took a test out of the run",
        full_description: "An added or changed line of the runner's settings stops an existing test file under the settings file's own directory from being collected, read from the settings file's kind and name rather than from any product. A setup, teardown or coverage setting chooses no test file. A line that names the file, or a pattern matching one, blocks; a line that narrows what the runner reads leaves the hidden set to be worked out, and warns. A setting whose value is built rather than written is reported unreadable instead of guessed at.",
        network: Network::None,
    },
    Rule {
        id: "T9",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A test case was added with no assertion",
        full_description: "A test case the change added asserts nothing: none of its own lines makes an assertion of its language's frameworks or calls a helper `[tests] assertions` names, it is not declared to expect a failure, it hands no failure back from a case declared to return one, and no function the same file defines that it calls asserts in its own body. Following a helper stops at that one level, so a helper of another file is named under `[tests] assertions`.",
        network: Network::None,
    },
    // T9 reads an added case with no assertion at all (issue #16). The added
    // cases that assert and still cannot fail (issue #107) take the next ids
    // in the order the issue names their shapes: T10 a constant restated, T11
    // a double asserted on alone, T12 a literal echoed back. The fourth shape,
    // a double of the unit the case is about, is M1's question asked of more
    // than the diff, so M1 answers it rather than an id of its own.
    Rule {
        id: "T10",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An added test restates a constant of the code",
        full_description: "A test case the change added checks nothing but a value the code defines against a literal: every assertion compares one name, read and not called, with a literal, and the name is the code's rather than the case's, imported by the file, defined by a production file outside the tests it carries, or reached without an import through Go's package scope or a glob import. A name the case binds, takes as a parameter or its own file defines is the test's own expectation and is left alone.",
        network: Network::None,
    },
    Rule {
        id: "T11",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An added test asserts only on its own double",
        full_description: "A test case the change added asserts on nothing but a double it built itself: every assertion reads a name the case bound to a double, made by a call or a type whose name says it makes one, or a value the case read off that double, and the case hands the double to nothing else, so no assertion reads what the code under test did. A double handed to the unit, with an assertion on the unit's output or on what the unit did to the double, is left alone.",
        network: Network::None,
    },
    Rule {
        id: "T12",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An added test only checks that its input came back",
        full_description: "A test case the change added checks nothing but that a string it passed in comes back out: every assertion holds what it reads to a string literal, outside the value it reads, that the case already wrote on an earlier line that is not a check, a prop, an argument or a fake's message. A literal of nothing but punctuation is no message and is passed over. A check that holds the answer to a literal of its own, the input with what the code adds to it, is left alone.",
        network: Network::None,
    },
    Rule {
        id: "M1",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A test mocks the unit it should exercise",
        full_description: "A test in the diff mocks a module whose production file is also in the diff, so the test no longer exercises the change it covers. A double the change sets up of what the test is about is reported too, whether the change edits that unit or not: a module double of the file the test file is named for, or a double of what the title of the added case it serves, or of a suite around that case, names.",
        network: Network::None,
    },
    Rule {
        id: "S1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A stub or a TODO reached production code",
        full_description: "An added line in a production file carries a stub marker, an unimplemented body, or a function body that only returns nothing. A body left unwritten in a base type states a contract rather than a stub, so the finding is dropped where production code somewhere in the tree is built on that type and writes the method, and reported at warn where nothing writes it and an entry file states the type, because the implementation may be in another repository.",
        network: Network::None,
    },
    Rule {
        id: "S2",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An error was swallowed",
        full_description: "An added handler in production code discards a failure without logging it, wrapping it, passing it on, or saying in a comment why it is right to drop it.",
        network: Network::None,
    },
    Rule {
        id: "S3",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A debug leftover reached production code",
        full_description: "An added line in a production file prints or breaks for debugging, outside the entry points the config names as command-line faces.",
        network: Network::None,
    },
    Rule {
        id: "S4",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "Production code was added that only its tests call",
        full_description: "A production file gains an exported definition, or exports one it already held, and every reference to it outside its own file is in a test, with at least one there. A reference is a call found anywhere in the tree, or the name written in the code of a file the change touched. The entry points their language exempts, the modules `[entrypoints] cli` names and the files `[entrypoints] loaded` names are left alone, and so is an export nothing references, which is R2's.",
        network: Network::None,
    },
    Rule {
        id: "D1",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A dependency manifest changed",
        full_description: "A package manifest or its lockfile changed in more than the package's own version; the change blocks when a scope excludes the manifest.",
        network: Network::None,
    },
    Rule {
        id: "D2",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "An import crossed a forbidden boundary",
        full_description: "A changed file imports across a direction that `[deps] allow` does not permit, by the layers `[deps] layers` names.",
        network: Network::None,
    },
    Rule {
        id: "X1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A secret-looking string was added",
        full_description: "An added line carries a known credential prefix, or a high-entropy literal assigned to a name that reads like a key, in source or in an env, YAML, TOML or INI file, where the literal is one run of characters rather than a sentence and reads nothing from the environment. A credential a vendor published in its own documentation is named as an example and reported as a note.",
        network: Network::None,
    },
    Rule {
        id: "X2",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A file outside the scope was touched",
        full_description: "A changed file matches no scope glob the run allows, and the finding names what depends on the definitions it changed.",
        network: Network::None,
    },
    Rule {
        id: "C1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A guardrail file was edited",
        full_description: "A change touched a harness settings file, a git hook, `weeder.toml`, the garden's `garden.lock` or `garden.json`, or a section of `AGENTS.md` or `CLAUDE.md` whose heading says hard limits or non-negotiables, or opens with never, or names one of `[guardrails] sections`. A release that moves only the product's own version in `garden.json`, its `version` and the `v<version>` segment of each asset URL, is not an edit.",
        network: Network::None,
    },
    Rule {
        id: "C2",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An ignore file was broadened over source or tests",
        full_description: "An added ignore pattern matches source or test paths of the repository's languages, hiding them from review and from tooling.",
        network: Network::None,
    },
    Rule {
        id: "C3",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A workflow was changed",
        full_description: "A file under `.github/workflows/` was added, edited or taken away; the change blocks where `[guardrails] paths` names the path.",
        network: Network::None,
    },
    Rule {
        id: "C4",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A gate was loosened in its own settings",
        full_description: "A step of a workflow under `.github/workflows/` or of a `.gitlab-ci.yml` that already ran a check was made unable to fail, by `continue-on-error: true`, `if: false` or `allow_failure: true` on the step or its job, or by `|| true`, `|| :` or `; exit 0` after the check's own command; or a `test`, `lint`, `typecheck` or `check` script of a manifest or a makefile, its `pre` and `post` hooks and `name:suffix` forms included, that ran a command now only prints or succeeds, answers its command with `|| true`, has make ignore its failure, or gains `--passWithNoTests`. Those block. A step is a check by the command its `run` or `script` names, read as a shell reads it, and never by the action it uses. A gate's own setting moved the loose way warns, read by its key and the table it sits in: a coverage threshold lowered, a strictness switch turned off or a mode moved off `strict`, a code added to what a lint ignores, a lint level lowered to `allow` or `off`. So does a check taken out of a workflow that runs on a push or a pull request, where no workflow the tree holds still runs it.",
        network: Network::None,
    },
    Rule {
        id: "G1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A conflict marker was committed",
        full_description: "An added line is a merge conflict marker, so the file carries both sides of a merge nobody finished.",
        network: Network::None,
    },
    Rule {
        id: "G2",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A large or a binary file was added",
        full_description: "An added file is larger than one mebibyte, or holds binary content, and was not tracked before.",
        network: Network::None,
    },
    Rule {
        id: "G3",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A control character was added to a text source file",
        full_description: "An added line carries a control character other than tab, newline or carriage return, in a file weeder reads as text by its path (source in a language it reads, a manifest, a guardrail, a workflow) or one git counted as text at the base. A character the change only carried over from a line it took out is not counted again. A NUL blocks, because git then takes the whole file for binary and shows no line of any change to it; every other control character warns.",
        network: Network::None,
    },
    Rule {
        id: "B1",
        face: Face::Bite,
        default_level: Level::Block,
        short_description: "A test passed without the change it covers",
        full_description: "The test command passed with the test commit alone applied to the base, so the cases that commit added were green before the implementation existed.",
        network: Network::None,
    },
    Rule {
        id: "R1",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "The docs cite something that no longer exists",
        full_description: "A path, command, flag or symbol cited in the repository's markdown does not resolve against the tree. A link is judged by its destination alone, read from the directory of the document that writes it.",
        network: Network::None,
    },
    Rule {
        id: "R2",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "A public symbol has no references",
        full_description: "An exported definition is referenced nowhere in the repository, and it is not one of the entry points its language exempts or in a file `[entrypoints] loaded` names.",
        network: Network::None,
    },
    Rule {
        id: "R3",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "A TODO is older than the configured age",
        full_description: "A line carrying `TODO`, `FIXME` or `XXX` was last touched further back than `[thresholds] todo_age_days` allows.",
        network: Network::None,
    },
    Rule {
        id: "R4",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "A dependency pin lags the registry",
        full_description: "A manifest pin is further behind the latest release than `[thresholds] dependency_lag` allows, measured against the committed registry snapshot. `weeder scan --refresh-snapshot` is the one path that reaches the registries, and it asks curl to do the reaching.",
        network: Network::Reaches {
            what: "registries",
            under: "--refresh-snapshot",
        },
    },
    Rule {
        id: "R5",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "A test file the configuration never collects",
        full_description: "A test file the tree holds is one the runner's settings never collect, named once per file. It is the tree-state twin of T8, so a repository that arrived with the exclusion already written is told as plainly as one that adds it.",
        network: Network::None,
    },
];

/// Every rule weeder knows, in catalogue order.
pub fn rules() -> &'static [Rule] {
    CATALOGUE
}

/// The catalogue entry for an id, or `None` for an id weeder does not know.
pub fn rule(id: &str) -> Option<&'static Rule> {
    CATALOGUE.iter().find(|rule| rule.id == id)
}

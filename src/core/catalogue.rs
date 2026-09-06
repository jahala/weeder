use crate::core::finding::Level;

/// The face a rule belongs to: `check` judges a diff, `scan` judges the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Check,
    Scan,
}

/// What a rule may reach over a network, and under which flag. weed is a judge:
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
    /// The one line `weed rules` prints in its network column.
    #[must_use]
    pub fn spelled(self) -> String {
        match self {
            Network::None => "none".to_string(),
            Network::Reaches { what, under } => format!("{what} under {under}"),
        }
    }
}

/// One rule as the catalogue knows it. This is the single source for the ids
/// `weed rules` prints, the defaults `Config::default()` ships, and the rules
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
        full_description: "A test file was removed, or a test case disappeared from a changed test file, counted from the file's classification and its test shape at HEAD against the working tree.",
        network: Network::None,
    },
    Rule {
        id: "T2",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "Assertions were dropped from a changed test file",
        full_description: "The assertion count of a changed test file fell between HEAD and the working tree, and no allowance carries a reason for it.",
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
        full_description: "Snapshot, golden or fixture files changed in a diff that also changes production code, so the expectation moved to meet the code.",
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
        id: "M1",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A test mocks the unit under change",
        full_description: "A test in the diff mocks a module whose production file is also in the diff, so the test no longer exercises the change it covers.",
        network: Network::None,
    },
    Rule {
        id: "S1",
        face: Face::Check,
        default_level: Level::Block,
        short_description: "A stub or a TODO reached production code",
        full_description: "An added line in a production file carries a stub marker, an unimplemented body, or a function body that only returns nothing.",
        network: Network::None,
    },
    Rule {
        id: "S2",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "An error was swallowed",
        full_description: "An added handler in production code discards a failure without logging it, wrapping it, or passing it on.",
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
        id: "D1",
        face: Face::Check,
        default_level: Level::Warn,
        short_description: "A dependency manifest changed",
        full_description: "A package manifest or its lockfile changed; the change blocks when a scope excludes the manifest.",
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
        full_description: "An added line carries a known credential prefix, or a high-entropy literal assigned to a name that reads like a key.",
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
        full_description: "A change touched a harness settings file, a git hook, `weed.toml`, or the hard-limits section of `AGENTS.md` or `CLAUDE.md`.",
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
        id: "R1",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "The docs cite something that no longer exists",
        full_description: "A path, command, flag or symbol cited in the repository's markdown does not resolve against the tree.",
        network: Network::None,
    },
    Rule {
        id: "R2",
        face: Face::Scan,
        default_level: Level::Warn,
        short_description: "A public symbol has no references",
        full_description: "An exported definition is referenced nowhere in the repository, and it is not one of the entry points its language exempts.",
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
        full_description: "A manifest pin is further behind the latest release than `[thresholds] dependency_lag` allows, measured against the committed registry snapshot. `weed scan --refresh-snapshot` is the one path that reaches the registries, and it asks curl to do the reaching.",
        network: Network::Reaches {
            what: "registries",
            under: "--refresh-snapshot",
        },
    },
];

/// Every rule weed knows, in catalogue order.
pub fn rules() -> &'static [Rule] {
    CATALOGUE
}

/// The catalogue entry for an id, or `None` for an id weed does not know.
pub fn rule(id: &str) -> Option<&'static Rule> {
    CATALOGUE.iter().find(|rule| rule.id == id)
}

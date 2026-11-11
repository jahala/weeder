use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::core::catalogue::{self, Face, Rule};
use crate::core::classify;
use crate::core::finding::Level;
use crate::core::pattern;

/// The name of the file a repository states its law in.
pub const CONFIG_FILE: &str = "weeder.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSetting {
    Off,
    Warn,
    Block,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDirection {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thresholds {
    pub todo_age_days: u32,
    pub dependency_lag: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `[rules]`: what the repository set each rule to, and only the rules it
    /// named. A rule it left out runs at its catalogue default, which
    /// [`Config::setting`] answers, and a level the repository did write caps
    /// every finding of that rule. It is only ever asked about a rule by id,
    /// never walked, so the order it holds its keys in reaches no output, the
    /// rules that run and the order they report in come from the catalogue,
    /// which is a slice.
    pub rules: HashMap<String, RuleSetting>,
    /// `[scope] allow`: the globs a change may touch; everything else is X2.
    pub scope_globs: Vec<String>,
    /// `[scope] exclude`: the path patterns no rule reads, as written. Each
    /// one that set a file aside is reported once, at note level, with a count,
    /// by whichever face skipped it.
    pub exclude: Vec<String>,
    /// `[[overrides]]`: rule levels for some paths only, in the order written.
    pub overrides: Vec<Override>,
    /// `[deps] layers`: layer name to the path globs that belong to it.
    pub layers: BTreeMap<String, Vec<String>>,
    /// `[deps] allow`: the import directions between layers that are permitted; D2 flags the rest.
    pub dependency_directions: Vec<DependencyDirection>,
    /// `[entrypoints] cli`: the modules that are the command line itself, where
    /// printing is the product rather than something left behind, and what they
    /// export is the command line's to call. S3 and S4 read these.
    pub cli_entrypoints: Vec<String>,
    /// `[entrypoints] loaded`: the path globs of files another program loads
    /// by the path it is given rather than by importing a name, a config file
    /// a tool reads, a spec a runner is pointed at. What they export is that
    /// program's to call, so R2 and S4 treat each as an entry point.
    pub loaded_entrypoints: Vec<String>,
    pub thresholds: Thresholds,
    /// `[docs] commands`: the commands R1 resolves a cited command and flag
    /// against, by reading each one's own `--help`. Empty leaves a cited
    /// command alone: weeder has no authority to resolve it against.
    pub doc_commands: Vec<String>,
    /// `[docs] external`: the names the docs cite on purpose that live outside
    /// the tree, a file written at run time, another program's environment
    /// variable. Each is a file name, a path or a glob of either, or a symbol,
    /// and R1 never reports a citation one of them covers.
    pub doc_external: Vec<String>,
    /// `[guard] protected`: branches guard refuses to rewrite or push non-fast-forward to.
    pub protected_branches: Vec<String>,
    /// `[guardrails] paths`: the path globs this repository holds at the
    /// constitution tier. weeder ships a set of those it recognises everywhere;
    /// this is where a repository names the ones only it can know. C3 reads it.
    pub guardrail_paths: Vec<String>,
    /// `[guardrails] sections`: the headings of `AGENTS.md` and `CLAUDE.md`
    /// a repository states its law under, beside the ones weeder recognises in
    /// every repository. C1 reads them.
    pub guardrail_sections: Vec<String>,
    /// `[tests] assertions`: the names of the helpers this repository's suites
    /// assert with, which no framework's vocabulary can know. A call to one is
    /// a claim wherever a test file makes it, in every language. T2 and T9 read
    /// them.
    pub test_assertions: Vec<String>,
}

/// One `[[overrides]]` table: the levels it writes for the rules it names,
/// under the paths it names. A level written here caps a finding under those
/// paths the way one written under `[rules]` caps it everywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override {
    pub paths: Vec<String>,
    pub rules: BTreeMap<String, RuleSetting>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub key: String,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.message)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// What a rule is set to: what the repository wrote for it, or the
    /// catalogue default where it wrote nothing.
    #[must_use]
    pub fn setting(&self, rule: &Rule) -> RuleSetting {
        self.rules
            .get(rule.id)
            .copied()
            .unwrap_or_else(|| default_setting(rule))
    }

    /// The level the repository wrote for a rule where a finding sits: the
    /// most specific `[[overrides]]` pattern that covers the path and names
    /// the rule, or what `[rules]` says, or `None` where it wrote nothing. Two
    /// equally specific patterns that disagree never get here; the parser
    /// refused them.
    #[must_use]
    pub fn written_at(&self, rule: &str, path: &str) -> Option<RuleSetting> {
        let mut best: Option<(usize, RuleSetting)> = None;
        for written in &self.overrides {
            let Some(&setting) = written.rules.get(rule) else {
                continue;
            };
            for candidate in &written.paths {
                let specificity = pattern::specificity(candidate);
                if pattern::covers(candidate, path)
                    && best.is_none_or(|(held, _)| specificity > held)
                {
                    best = Some((specificity, setting));
                }
            }
        }
        best.map(|(_, setting)| setting)
            .or_else(|| self.rules.get(rule).copied())
    }

    /// Whether a rule is on for any path: `[rules]` leaves it on, or an
    /// override turns it on somewhere. A rule off everywhere is not run.
    #[must_use]
    pub fn on_anywhere(&self, rule: &Rule) -> bool {
        self.rules.get(rule.id) != Some(&RuleSetting::Off)
            || self.overrides.iter().any(|written| {
                written
                    .rules
                    .get(rule.id)
                    .is_some_and(|setting| *setting != RuleSetting::Off)
            })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Nothing stated. Asked by id only, so its order reaches no output.
            rules: HashMap::new(),
            scope_globs: vec!["**/*".to_string()],
            exclude: Vec::new(),
            overrides: Vec::new(),
            layers: BTreeMap::new(),
            dependency_directions: Vec::new(),
            cli_entrypoints: Vec::new(),
            loaded_entrypoints: Vec::new(),
            thresholds: Thresholds {
                todo_age_days: 30,
                dependency_lag: 3,
            },
            doc_commands: Vec::new(),
            doc_external: Vec::new(),
            protected_branches: vec!["main".to_string(), "master".to_string()],
            guardrail_paths: Vec::new(),
            guardrail_sections: Vec::new(),
            test_assertions: Vec::new(),
        }
    }
}

pub fn parse_config(input: Option<&str>) -> Result<Config, ConfigError> {
    let Some(input) = input else {
        return Ok(Config::default());
    };
    let raw: RawConfig = toml::from_str(input).map_err(|error| ConfigError {
        key: CONFIG_FILE.to_string(),
        message: error.message().to_string(),
    })?;
    let mut config = Config::default();

    if let Some(rules) = raw.rules {
        for (rule, value) in rules {
            if catalogue::rule(&rule).is_none() {
                return Err(unknown_rule(&rule));
            }
            let setting = parse_rule_setting(&rule, &value)?;
            config.rules.insert(rule, setting);
        }
    }
    if let Some(scope) = raw.scope {
        if let Some(allow) = scope.allow {
            config.scope_globs = allow;
        }
        if let Some(exclude) = scope.exclude {
            config.exclude = exclude;
        }
    }
    if let Some(deps) = raw.deps {
        if let Some(layers) = deps.layers {
            config.layers = layers;
        }
        config.dependency_directions = deps
            .allow
            .unwrap_or_default()
            .into_iter()
            .map(|direction| DependencyDirection {
                from: direction.from,
                to: direction.to,
            })
            .collect();
        for direction in &config.dependency_directions {
            for layer in [&direction.from, &direction.to] {
                if !config.layers.contains_key(layer) {
                    return Err(ConfigError {
                        key: format!("deps.allow.{layer}"),
                        message: "names a layer that [deps] layers does not define".to_string(),
                    });
                }
            }
        }
    }
    if let Some(entrypoints) = raw.entrypoints {
        if let Some(cli) = entrypoints.cli {
            config.cli_entrypoints = cli;
        }
        if let Some(loaded) = entrypoints.loaded {
            config.loaded_entrypoints = loaded;
        }
    }
    if let Some(thresholds) = raw.thresholds {
        if let Some(value) = thresholds.todo_age_days {
            config.thresholds.todo_age_days = value;
        }
        if let Some(value) = thresholds.dependency_lag {
            config.thresholds.dependency_lag = value;
        }
    }
    if let Some(docs) = raw.docs {
        if let Some(commands) = docs.commands {
            for command in &commands {
                if command.trim().is_empty() || command.split_whitespace().count() != 1 {
                    return Err(ConfigError {
                        key: "docs.commands".to_string(),
                        message: format!(
                            "`{command}` is not a command weeder can run. name the program alone; weeder passes its own arguments and never goes through a shell"
                        ),
                    });
                }
            }
            config.doc_commands = commands;
        }
        if let Some(external) = docs.external {
            if let Some(blank) = external.iter().find(|name| name.trim().is_empty()) {
                return Err(ConfigError {
                    key: "docs.external".to_string(),
                    message: format!(
                        "`{blank}` names nothing. write the file name, path, glob or variable the docs cite"
                    ),
                });
            }
            config.doc_external = external;
        }
    }
    if let Some(guard) = raw.guard {
        if let Some(branches) = guard.protected {
            config.protected_branches = branches;
        }
    }
    if let Some(guardrails) = raw.guardrails {
        if let Some(paths) = guardrails.paths {
            config.guardrail_paths = paths;
        }
        if let Some(sections) = guardrails.sections {
            if let Some(blank) = sections.iter().find(|title| title.trim().is_empty()) {
                return Err(ConfigError {
                    key: "guardrails.sections".to_string(),
                    message: format!(
                        "`{blank}` names no heading. write the words the heading of the guarded section says"
                    ),
                });
            }
            config.guardrail_sections = sections;
        }
    }
    if let Some(tests) = raw.tests {
        if let Some(assertions) = tests.assertions {
            if let Some(blank) = assertions.iter().find(|name| name.trim().is_empty()) {
                return Err(ConfigError {
                    key: "tests.assertions".to_string(),
                    message: format!(
                        "`{blank}` names no helper. write the name a test file calls the helper by"
                    ),
                });
            }
            config.test_assertions = assertions;
        }
    }

    if let Some(overrides) = raw.overrides {
        config.overrides = overrides
            .into_iter()
            .map(parse_override)
            .collect::<Result<Vec<Override>, ConfigError>>()?;
    }
    patterns_allowed(&config)?;

    Ok(config)
}

/// One `[[overrides]]` table, its rule ids and levels read the way `[rules]`
/// reads them.
fn parse_override(raw: RawOverride) -> Result<Override, ConfigError> {
    if raw.paths.is_empty() || raw.rules.is_empty() {
        return Err(ConfigError {
            key: "overrides".to_string(),
            message: "an override with no paths or no rules sets nothing. name the paths it applies to and a level for each rule it changes there".to_string(),
        });
    }
    let mut rules = BTreeMap::new();
    for (rule, value) in raw.rules {
        if catalogue::rule(&rule).is_none() {
            let mut refused = unknown_rule(&rule);
            refused.key = format!("overrides.{}", refused.key);
            return Err(refused);
        }
        let setting = parse_rule_setting(&rule, &value)?;
        rules.insert(rule, setting);
    }
    Ok(Override {
        paths: raw.paths,
        rules,
    })
}

/// The refusals every path pattern answers to, `[scope] exclude` and
/// `[[overrides]]` alike, and the one only overrides can earn: two patterns as
/// specific as each other that can cover one file and set one rule two ways.
/// weeder will not pick between them, because either pick is a level nobody
/// wrote for that file.
fn patterns_allowed(config: &Config) -> Result<(), ConfigError> {
    let guardrails: Vec<&str> = classify::GUARDRAILS
        .iter()
        .chain(classify::WORKFLOWS)
        .copied()
        .chain(config.guardrail_paths.iter().map(String::as_str))
        .collect();
    let written = config
        .exclude
        .iter()
        .map(|written| ("scope.exclude", written))
        .chain(
            config
                .overrides
                .iter()
                .flat_map(|one| one.paths.iter().map(|written| ("overrides.paths", written))),
        );
    for (key, written) in written {
        if let Some(refused) = pattern::refusal(written, &guardrails) {
            return Err(ConfigError {
                key: key.to_string(),
                message: refused,
            });
        }
    }
    for (index, one) in config.overrides.iter().enumerate() {
        for other in &config.overrides[index + 1..] {
            if let Some(refused) = conflict(one, other) {
                return Err(ConfigError {
                    key: "overrides".to_string(),
                    message: refused,
                });
            }
        }
    }
    Ok(())
}

/// Where two overrides tie for one file and disagree about one rule, what to
/// say about it.
fn conflict(one: &Override, other: &Override) -> Option<String> {
    for (rule, setting) in &one.rules {
        let Some(theirs) = other.rules.get(rule).filter(|theirs| *theirs != setting) else {
            continue;
        };
        for left in &one.paths {
            for right in &other.paths {
                if pattern::specificity(left) == pattern::specificity(right)
                    && pattern::meet(left, right)
                {
                    return Some(format!(
                        "`{left}` and `{right}` are as specific as each other, can cover the same file, and set {rule} to {} and to {}. weeder will not pick one: make one pattern narrower, or set {rule} the same in both",
                        word(*setting),
                        word(*theirs)
                    ));
                }
            }
        }
    }
    None
}

/// A setting as `weeder.toml` spells it.
fn word(setting: RuleSetting) -> &'static str {
    match setting {
        RuleSetting::Off => "off",
        RuleSetting::Warn => "warn",
        RuleSetting::Block => "block",
        RuleSetting::On => "on",
    }
}

/// The refusal of a `[rules]` key the catalogue does not hold. Such a key is
/// taken and changes nothing, so a repository that meant to turn a rule off or
/// up would believe it had; the error names the id weeder thinks was meant.
fn unknown_rule(key: &str) -> ConfigError {
    let message = match catalogue::rules()
        .iter()
        .find(|rule| rule.id.eq_ignore_ascii_case(key))
    {
        Some(rule) => format!(
            "rule ids are written in capitals, so this sets nothing. write it as `{}`",
            rule.id
        ),
        None => format!(
            "weeder has no rule of that id, so this sets nothing. the nearest id it has is `{}`, and `weeder rules` lists them all",
            nearest(key)
        ),
    };
    ConfigError {
        key: format!("rules.{key}"),
        message,
    }
}

/// The catalogue id spelled nearest to a key: the fewest characters changed,
/// then an id that shares the key's first character, then the catalogue's own
/// order, so the answer is the same on every run.
fn nearest(key: &str) -> &'static str {
    let key = key.to_ascii_uppercase();
    let first = key.chars().next();
    catalogue::rules()
        .iter()
        .min_by_key(|rule| (distance(&key, rule.id), rule.id.chars().next() != first))
        .map_or("", |rule| rule.id)
}

/// How many characters must be inserted, deleted or replaced to turn one
/// string into the other.
fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, leftward) in left.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, rightward) in right.iter().enumerate() {
            let replace = previous[column] + usize::from(leftward != *rightward);
            let insert = current[column] + 1;
            let delete = previous[column + 1] + 1;
            current.push(replace.min(insert).min(delete));
        }
        previous = current;
    }
    previous[right.len()]
}

fn parse_rule_setting(rule: &str, value: &str) -> Result<RuleSetting, ConfigError> {
    let lower = value.to_ascii_lowercase();
    if is_scan_rule(rule) {
        match lower.as_str() {
            "off" => Ok(RuleSetting::Off),
            "on" => Ok(RuleSetting::On),
            _ => Err(ConfigError {
                key: rule.to_string(),
                message: "scan rules accept off or on".to_string(),
            }),
        }
    } else {
        match lower.as_str() {
            "off" => Ok(RuleSetting::Off),
            "warn" => Ok(RuleSetting::Warn),
            "block" => Ok(RuleSetting::Block),
            _ => Err(ConfigError {
                key: rule.to_string(),
                message: "check rules accept off, warn, or block".to_string(),
            }),
        }
    }
}

/// What the catalogue's default level means as a config setting: scan rules are
/// on or off, the rules that judge a change carry a level.
fn default_setting(rule: &Rule) -> RuleSetting {
    match rule.face {
        Face::Scan => RuleSetting::On,
        Face::Check | Face::Bite => match rule.default_level {
            Level::Block => RuleSetting::Block,
            Level::Warn | Level::Note => RuleSetting::Warn,
        },
    }
}

fn is_scan_rule(rule: &str) -> bool {
    catalogue::rule(rule).is_some_and(|rule| rule.face == Face::Scan)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    rules: Option<BTreeMap<String, String>>,
    scope: Option<RawScope>,
    deps: Option<RawDeps>,
    entrypoints: Option<RawEntrypoints>,
    thresholds: Option<RawThresholds>,
    docs: Option<RawDocs>,
    guard: Option<RawGuard>,
    guardrails: Option<RawGuardrails>,
    tests: Option<RawTests>,
    overrides: Option<Vec<RawOverride>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverride {
    paths: Vec<String>,
    rules: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScope {
    allow: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeps {
    layers: Option<BTreeMap<String, Vec<String>>>,
    allow: Option<Vec<DependencyDirectionToml>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencyDirectionToml {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntrypoints {
    cli: Option<Vec<String>>,
    loaded: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawThresholds {
    todo_age_days: Option<u32>,
    dependency_lag: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDocs {
    commands: Option<Vec<String>>,
    external: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGuard {
    protected: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGuardrails {
    paths: Option<Vec<String>>,
    sections: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTests {
    assertions: Option<Vec<String>>,
}

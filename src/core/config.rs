use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::core::catalogue::{self, Face, Rule};
use crate::core::finding::Level;
use crate::core::specimen;

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
    /// `[scope] specimens`: the fixture directories no rule reads. Every path
    /// one of them covers is reported once, at note level, by whichever face
    /// skipped it.
    pub specimens: Vec<String>,
    /// `[deps] layers`: layer name to the path globs that belong to it.
    pub layers: BTreeMap<String, Vec<String>>,
    /// `[deps] allow`: the import directions between layers that are permitted; D2 flags the rest.
    pub dependency_directions: Vec<DependencyDirection>,
    /// `[entrypoints] cli`: the modules that are the command line itself, where
    /// printing is the product rather than something left behind. S3 reads these.
    pub cli_entrypoints: Vec<String>,
    pub thresholds: Thresholds,
    /// `[docs] commands`: the commands R1 resolves a cited command and flag
    /// against, by reading each one's own `--help`. Empty leaves a cited
    /// command alone: weeder has no authority to resolve it against.
    pub doc_commands: Vec<String>,
    /// `[guard] protected`: branches guard refuses to rewrite or push non-fast-forward to.
    pub protected_branches: Vec<String>,
    /// `[guardrails] paths`: the path globs this repository holds at the
    /// constitution tier. weeder ships a set of those it recognises everywhere;
    /// this is where a repository names the ones only it can know. C3 reads it.
    pub guardrail_paths: Vec<String>,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Nothing stated. Asked by id only, so its order reaches no output.
            rules: HashMap::new(),
            scope_globs: vec!["**/*".to_string()],
            specimens: Vec::new(),
            layers: BTreeMap::new(),
            dependency_directions: Vec::new(),
            cli_entrypoints: Vec::new(),
            thresholds: Thresholds {
                todo_age_days: 30,
                dependency_lag: 3,
            },
            doc_commands: Vec::new(),
            protected_branches: vec!["main".to_string(), "master".to_string()],
            guardrail_paths: Vec::new(),
        }
    }
}

pub fn parse_config(input: Option<&str>) -> Result<Config, ConfigError> {
    let Some(input) = input else {
        return Ok(Config::default());
    };
    let raw: RawConfig = toml::from_str(input).map_err(|error| ConfigError {
        key: "weeder.toml".to_string(),
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
        if let Some(specimens) = scope.specimens {
            for entry in &specimens {
                if let Some(refused) = specimen::refusal(entry) {
                    return Err(ConfigError {
                        key: "scope.specimens".to_string(),
                        message: refused,
                    });
                }
            }
            config.specimens = specimens;
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
    }

    Ok(config)
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScope {
    allow: Option<Vec<String>>,
    specimens: Option<Vec<String>>,
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
}

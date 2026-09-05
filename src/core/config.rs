use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::core::catalogue::{self, Face, Rule};
use crate::core::finding::Level;

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
    /// `[rules]`: what the repository set each rule to. It is only ever asked
    /// about a rule by id, never walked, so the order it holds its keys in
    /// reaches no output — the rules that run and the order they report in come
    /// from the catalogue, which is a slice.
    pub rules: HashMap<String, RuleSetting>,
    /// `[scope] allow`: the globs a change may touch; everything else is X2.
    pub scope_globs: Vec<String>,
    /// `[deps] layers`: layer name to the path globs that belong to it.
    pub layers: BTreeMap<String, Vec<String>>,
    /// `[deps] allow`: the import directions between layers that are permitted; D2 flags the rest.
    pub dependency_directions: Vec<DependencyDirection>,
    pub thresholds: Thresholds,
    /// `[guard] protected`: branches guard refuses to rewrite or push non-fast-forward to.
    pub protected_branches: Vec<String>,
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

impl Default for Config {
    fn default() -> Self {
        let rules = catalogue::rules()
            .iter()
            .map(|rule| (rule.id.to_string(), default_setting(rule)))
            .collect();

        Self {
            rules,
            scope_globs: vec!["**/*".to_string()],
            layers: BTreeMap::new(),
            dependency_directions: Vec::new(),
            thresholds: Thresholds {
                todo_age_days: 30,
                dependency_lag: 3,
            },
            protected_branches: vec!["main".to_string(), "master".to_string()],
        }
    }
}

pub fn parse_config(input: Option<&str>) -> Result<Config, ConfigError> {
    let Some(input) = input else {
        return Ok(Config::default());
    };
    let raw: RawConfig = toml::from_str(input).map_err(|error| ConfigError {
        key: "weed.toml".to_string(),
        message: error.message().to_string(),
    })?;
    let mut config = Config::default();

    if let Some(rules) = raw.rules {
        for (rule, value) in rules {
            let setting = parse_rule_setting(&rule, &value)?;
            config.rules.insert(rule, setting);
        }
    }
    if let Some(scope) = raw.scope {
        if let Some(allow) = scope.allow {
            config.scope_globs = allow;
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
    if let Some(thresholds) = raw.thresholds {
        if let Some(value) = thresholds.todo_age_days {
            config.thresholds.todo_age_days = value;
        }
        if let Some(value) = thresholds.dependency_lag {
            config.thresholds.dependency_lag = value;
        }
    }
    if let Some(guard) = raw.guard {
        if let Some(branches) = guard.protected {
            config.protected_branches = branches;
        }
    }

    Ok(config)
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
/// on or off, check rules carry a level.
fn default_setting(rule: &Rule) -> RuleSetting {
    match rule.face {
        Face::Scan => RuleSetting::On,
        Face::Check => match rule.default_level {
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
    thresholds: Option<RawThresholds>,
    guard: Option<RawGuard>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScope {
    allow: Option<Vec<String>>,
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
struct RawThresholds {
    todo_age_days: Option<u32>,
    dependency_lag: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGuard {
    protected: Option<Vec<String>>,
}

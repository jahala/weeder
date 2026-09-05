use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

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
    pub rules: HashMap<String, RuleSetting>,
    pub scope_globs: Vec<String>,
    pub dependency_directions: Vec<DependencyDirection>,
    pub thresholds: Thresholds,
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
        let mut rules = HashMap::new();
        for rule in ["T1", "T2", "T3", "T7", "S1", "D2", "X1", "X2", "C1", "G1"] {
            rules.insert(rule.to_string(), RuleSetting::Block);
        }
        for rule in ["T4", "T5", "T6", "M1", "S2", "S3", "D1", "C2", "G2"] {
            rules.insert(rule.to_string(), RuleSetting::Warn);
        }
        for rule in ["R1", "R2", "R3", "R4"] {
            rules.insert(rule.to_string(), RuleSetting::On);
        }

        Self {
            rules,
            scope_globs: vec!["**/*".to_string()],
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
        key: toml_error_key(&error),
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
        config.scope_globs = scope.allowed_globs();
    }
    if let Some(deps) = raw.deps {
        config.dependency_directions = deps.directions();
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
        if let Some(branches) = guard.protected_branches.or(guard.branches) {
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

fn is_scan_rule(rule: &str) -> bool {
    matches!(rule, "R1" | "R2" | "R3" | "R4")
}

fn toml_error_key(error: &toml::de::Error) -> String {
    error
        .span()
        .map(|_| "weed.toml".to_string())
        .unwrap_or_else(|| "weed.toml".to_string())
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
    allowed: Option<Vec<String>>,
    globs: Option<Vec<String>>,
}

impl RawScope {
    fn allowed_globs(self) -> Vec<String> {
        self.allowed.or(self.globs).unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeps {
    directions: Option<Vec<DependencyDirectionToml>>,
    allowed: Option<Vec<DependencyDirectionToml>>,
}

impl RawDeps {
    fn directions(self) -> Vec<DependencyDirection> {
        self.directions
            .or(self.allowed)
            .unwrap_or_default()
            .into_iter()
            .map(|direction| DependencyDirection {
                from: direction.from,
                to: direction.to,
            })
            .collect()
    }
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
    protected_branches: Option<Vec<String>>,
    branches: Option<Vec<String>>,
}

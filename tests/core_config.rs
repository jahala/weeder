use weed::core::{parse_config, RuleSetting};

#[test]
fn parses_typed_config_and_defaults() {
    let defaults = parse_config(None).expect("absent config should use defaults");
    assert_eq!(defaults.rules["T1"], RuleSetting::Block);
    assert_eq!(defaults.rules["T4"], RuleSetting::Warn);
    assert_eq!(defaults.rules["R1"], RuleSetting::On);
    assert_eq!(defaults.thresholds.todo_age_days, 30);
    assert!(defaults
        .protected_branches
        .iter()
        .any(|branch| branch == "main"));

    let config = parse_config(Some(
        r#"
[rules]
T1 = "warn"
T4 = "off"
R1 = "on"
R2 = "off"

[scope]
allowed = ["src/**", "tests/**"]

[deps]
directions = [
  { from = "faces", to = "core" },
  { from = "seams", to = "core" },
]

[thresholds]
todo_age_days = 14
dependency_lag = 5

[guard]
protected_branches = ["main", "release"]
"#,
    ))
    .expect("valid config should parse");

    assert_eq!(config.rules["T1"], RuleSetting::Warn);
    assert_eq!(config.rules["T4"], RuleSetting::Off);
    assert_eq!(config.rules["R2"], RuleSetting::Off);
    assert_eq!(config.scope_globs, vec!["src/**", "tests/**"]);
    assert_eq!(config.dependency_directions[0].from, "faces");
    assert_eq!(config.dependency_directions[0].to, "core");
    assert_eq!(config.thresholds.todo_age_days, 14);
    assert_eq!(config.thresholds.dependency_lag, 5);
    assert_eq!(config.protected_branches, vec!["main", "release"]);
}

#[test]
fn config_errors_name_bad_keys_and_levels() {
    let unknown = parse_config(Some("unknown = true")).expect_err("unknown keys must fail");
    assert!(unknown.to_string().contains("unknown"));

    let invalid = parse_config(Some("[rules]\nT1 = \"loud\"\n")).expect_err("bad level must fail");
    assert!(invalid.to_string().contains("T1"));

    let scan_block =
        parse_config(Some("[rules]\nR1 = \"block\"\n")).expect_err("scan block must fail");
    assert!(scan_block.to_string().contains("R1"));
}

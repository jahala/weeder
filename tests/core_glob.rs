//! The globs `[scope] allow` and `--scope` are written in.

use weeder::core::glob::{matches, matches_any};

#[test]
fn a_single_star_stops_at_a_separator() {
    assert!(matches("*.ts", "tracked.ts"));
    assert!(
        !matches("*.ts", "src/nested.ts"),
        "a single star never reaches into src/"
    );
    assert!(matches("src/*.ts", "src/nested.ts"));
    assert!(!matches("src/*.ts", "src/deep/nested.ts"));
}

#[test]
fn a_double_star_crosses_separators_and_may_match_nothing() {
    assert!(matches("**/*.ts", "src/nested.ts"));
    assert!(
        matches("**/*.ts", "tracked.ts"),
        "**/ also swallows no segment at all"
    );
    assert!(matches("src/**", "src/deep/nested.ts"));
    assert!(!matches("src/**", "tests/nested.ts"));
}

#[test]
fn a_question_mark_is_one_character_that_is_not_a_separator() {
    assert!(matches("src/?.ts", "src/a.ts"));
    assert!(!matches("src/?.ts", "src/ab.ts"));
    assert!(!matches("src?a.ts", "src/a.ts"));
}

#[test]
fn any_of_several_globs_allows_a_path_and_none_allows_nothing() {
    let globs = vec!["**/*.ts".to_string(), "docs/**".to_string()];
    assert!(matches_any(&globs, "src/nested.ts"));
    assert!(matches_any(&globs, "docs/guide.md"));
    assert!(!matches_any(&globs, "Cargo.toml"));
    assert!(
        !matches_any(&[], "anything"),
        "an empty list allows nothing"
    );
}

//! The globs `[scope] allow` and `--scope` are written in.

use weeder::core::glob::{matches, matches_any, overlap};

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

#[test]
fn two_globs_overlap_where_one_path_matches_both() {
    for (left, right, path) in [
        ("docs/**", "**/*.md", "docs/guide.md"),
        ("docs/*", "*/keep", "docs/keep"),
        ("**/*.ts", "src/*.ts", "src/a.ts"),
        ("src/?.ts", "src/a*", "src/a.ts"),
        (
            ".github/workflows/**",
            ".github/**",
            ".github/workflows/ci.yml",
        ),
        ("**/x", "x", "x"),
    ] {
        assert!(
            matches(left, path) && matches(right, path),
            "{path} matches both, or this case proves nothing"
        );
        assert!(overlap(left, right), "{left} and {right} meet at {path}");
        assert!(overlap(right, left), "and the answer has no direction");
    }
}

#[test]
fn two_globs_that_no_path_matches_together_do_not_overlap() {
    for (left, right) in [
        ("docs/**", "test/**"),
        ("*.ts", "src/*.ts"),
        ("src/*", "src/a/b"),
        ("**/*.md", "**/*.ts"),
        ("src/?.ts", "src/ab.ts"),
        ("a", "b"),
    ] {
        assert!(
            !overlap(left, right),
            "{left} and {right} name no common path"
        );
        assert!(!overlap(right, left), "either way round");
    }
}

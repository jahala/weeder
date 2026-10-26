//! `docs/rules.md`, the page every SARIF result links back to.
//!
//! weeder writes a `helpUri` of `docs/rules.md#<id>` for every rule it knows, so
//! the page has to carry an anchor of that id, and to say about the rule what
//! the binary says about it. This test reads the catalogue and the page against
//! each other in both directions: a rule the page does not carry, and an anchor
//! the catalogue has never heard of, both fail here. The page's table of what
//! `weeder.toml` sets is held to the parser the same way, so a key weeder
//! accepts and the page leaves out fails here too.

use std::path::Path;

use weeder::core::{catalogue, parse_config};

/// The page the SARIF `helpUri` points at.
const DOC: &str = "docs/rules.md";

fn page() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(DOC);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
}

#[test]
fn every_rule_has_an_anchor_and_says_what_the_catalogue_says() {
    let page = page();
    for rule in catalogue::rules() {
        let anchor = format!("<a id=\"{}\"></a>", rule.id);
        assert!(
            page.contains(&anchor),
            "{DOC} carries no anchor for {}, so its helpUri lands nowhere",
            rule.id
        );
        assert!(
            page.contains(rule.short_description),
            "{DOC} does not say what {} reports: {}",
            rule.id,
            rule.short_description
        );
        assert!(
            page.contains(rule.full_description),
            "{DOC} does not say what {} reads to decide",
            rule.id
        );
    }
}

#[test]
fn the_page_names_no_rule_the_catalogue_does_not_have() {
    let page = page();
    let anchored: Vec<String> = page
        .match_indices("<a id=\"")
        .filter_map(|(at, opening)| {
            let rest = &page[at + opening.len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        })
        .collect();
    assert_eq!(
        anchored.len(),
        catalogue::rules().len(),
        "one anchor per rule, and no rule anchored twice"
    );
    for id in &anchored {
        assert!(
            catalogue::rule(id).is_some(),
            "{DOC} documents `{id}`, which weeder does not have"
        );
    }
}

/// The keys `weeder.toml` accepts where `toml` sets one it does not, read off
/// the refusal itself: the parser names every field it expected, so the list
/// comes from the binary and no copy of it is kept here.
fn expected_keys(toml: &str) -> Vec<String> {
    let refusal = parse_config(Some(toml))
        .expect_err("a key weeder does not read is refused")
        .to_string();
    let (_, expected) = refusal
        .split_once("expected")
        .unwrap_or_else(|| panic!("the refusal names what was expected: {refusal}"));
    expected
        .split('`')
        .skip(1)
        .step_by(2)
        .map(ToString::to_string)
        .collect()
}

#[test]
fn the_config_table_lists_every_key_weeder_toml_accepts() {
    let page = page();
    let sections = expected_keys("not_a_section = 1\n");
    assert!(
        sections.iter().any(|section| section == "docs"),
        "the sections were read off the refusal: {sections:?}"
    );
    for section in sections {
        // `[rules]` is a table of rule ids, and the tests above hold the page
        // to every one of them, so its row is the section alone.
        if section == "rules" {
            assert!(
                page.contains("| `[rules]` |"),
                "{DOC} lists no `[rules]` row"
            );
            continue;
        }
        for key in expected_keys(&format!("[{section}]\nnot_a_key = 1\n")) {
            assert!(
                page.contains(&format!("| `[{section}] {key}` |")),
                "{DOC} does not list `[{section}] {key}`, which weeder.toml accepts"
            );
        }
    }
}

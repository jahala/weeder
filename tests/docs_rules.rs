//! `docs/rules.md`, the page every SARIF result links back to.
//!
//! weed writes a `helpUri` of `docs/rules.md#<id>` for every rule it knows, so
//! the page has to carry an anchor of that id, and to say about the rule what
//! the binary says about it. This test reads the catalogue and the page against
//! each other in both directions: a rule the page does not carry, and an anchor
//! the catalogue has never heard of, both fail here.

use std::path::Path;

use weed::core::catalogue;

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
            "{DOC} documents `{id}`, which weed does not have"
        );
    }
}

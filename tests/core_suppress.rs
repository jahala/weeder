use weeder::core::{
    apply_suppressions, parse_commit_suppressions, parse_diff, parse_inline_suppressions, Finding,
    Level, Message, Region, SuppressionSource,
};

/// An allowance, built rather than spelled. weeder judges its own tree, and a
/// line here that spells one whole is an allowance in weeder's own diff: the
/// same reason the fixture harness builds its conflict markers instead of
/// carrying them.
fn allowance(rule: &str, reason: &str) -> String {
    let token = format!("weeder{}allow", "-");
    if reason.is_empty() {
        format!("// {token} {rule}")
    } else {
        format!("// {token} {rule}: {reason}")
    }
}

#[test]
fn parses_commit_and_inline_suppressions_with_reasons() {
    let commit = "Subject\n\nWeeder-allow: T2 legacy test shape\nWeeder-allow: T3\n";
    let commit_suppressions = parse_commit_suppressions(commit);
    assert_eq!(commit_suppressions.len(), 1);
    assert_eq!(commit_suppressions[0].rule, "T2");
    assert_eq!(commit_suppressions[0].reason, "legacy test shape");
    assert_eq!(
        commit_suppressions[0].source,
        SuppressionSource::CommitTrailer
    );

    let diff = parse_diff(&format!(
        "diff --git a/tests/a.test.ts b/tests/a.test.ts\n\
         index 1111111..2222222 100644\n\
         --- a/tests/a.test.ts\n\
         +++ b/tests/a.test.ts\n\
         @@ -1,2 +1,4 @@\n\
         +{focused}\n\
         +it.only(\"runs\", () => {{}})\n\
         +{unreadable}\n\
         \x20it(\"keeps running\", () => {{}})\n",
        focused = allowance("T3", "third-party suite still carries focus"),
        unreadable = allowance("T2", ""),
    ))
    .expect("diff parses");
    let (inline, errors) = parse_inline_suppressions(&diff);
    assert_eq!(inline.len(), 1);
    assert_eq!(inline[0].rule, "T3");
    assert_eq!(inline[0].path.as_deref(), Some("tests/a.test.ts"));
    assert_eq!(inline[0].line, Some(1));
    assert_eq!(inline[0].source, SuppressionSource::InlineComment);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].rule, "T2");
}

#[test]
fn matching_suppression_turns_finding_into_note_with_reason_and_original_level() {
    let diff = parse_diff(&format!(
        "diff --git a/tests/a.test.ts b/tests/a.test.ts\n\
         index 1111111..2222222 100644\n\
         --- a/tests/a.test.ts\n\
         +++ b/tests/a.test.ts\n\
         @@ -1 +1,2 @@\n\
         +{quarantined}\n\
         \x20it.only(\"runs\", () => {{}})\n",
        quarantined = allowance("T3", "quarantined upstream case"),
    ))
    .expect("diff parses");
    let (suppressions, _) = parse_inline_suppressions(&diff);
    let findings = vec![Finding {
        rule: "T3".to_string(),
        level: Level::Block,
        path: "tests/a.test.ts".to_string(),
        region: Some(Region {
            start_line: 2,
            end_line: 2,
        }),
        message: Message {
            what: "A focused test was added.".to_string(),
            why: "Focused tests hide the rest of the suite.".to_string(),
            next: "Remove the focus marker or carry an allowance with a reason.".to_string(),
        },
        fix: None,
        suppressed: None,
    }];

    let applied = apply_suppressions(findings, &suppressions);
    assert_eq!(applied[0].level, Level::Note);
    let suppression = applied[0]
        .suppressed
        .as_ref()
        .expect("suppression is attached");
    assert_eq!(suppression.reason, "quarantined upstream case");
    assert_eq!(suppression.original_level, Some(Level::Block));
}

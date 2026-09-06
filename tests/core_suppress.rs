use weed::core::{
    apply_suppressions, parse_commit_suppressions, parse_diff, parse_inline_suppressions, Finding,
    Level, Message, Region, SuppressionSource,
};

#[test]
fn parses_commit_and_inline_suppressions_with_reasons() {
    let commit = "Subject\n\nWeed-allow: T2 legacy test shape\nWeed-allow: T3\n";
    let commit_suppressions = parse_commit_suppressions(commit);
    assert_eq!(commit_suppressions.len(), 1);
    assert_eq!(commit_suppressions[0].rule, "T2");
    assert_eq!(commit_suppressions[0].reason, "legacy test shape");
    assert_eq!(
        commit_suppressions[0].source,
        SuppressionSource::CommitTrailer
    );

    let diff = parse_diff(
        r#"diff --git a/tests/a.test.ts b/tests/a.test.ts
index 1111111..2222222 100644
--- a/tests/a.test.ts
+++ b/tests/a.test.ts
@@ -1,2 +1,4 @@
+// weed-allow T3: third-party suite still carries focus
+it.only("runs", () => {})
+// weed-allow T2
 it("keeps running", () => {})
"#,
    )
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
    let diff = parse_diff(
        r#"diff --git a/tests/a.test.ts b/tests/a.test.ts
index 1111111..2222222 100644
--- a/tests/a.test.ts
+++ b/tests/a.test.ts
@@ -1 +1,2 @@
+// weed-allow T3: quarantined upstream case
 it.only("runs", () => {})
"#,
    )
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

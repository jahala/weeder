use demo::contrast;

#[test]
fn text_on_paper_reads_in_light() {
    assert!(contrast("light", "text", "paper") >= 4.5);
}

#[test]
fn muted_on_paper_reads_in_light() {
    assert!(contrast("light", "muted", "paper") >= 4.5);
}

#[test]
fn edge_on_paper_shows_in_light() {
    assert!(contrast("light", "edge", "paper") >= 3.0);
}

#[test]
fn focus_on_paper_shows_in_light() {
    assert!(contrast("light", "focus", "paper") >= 3.0);
}

#[test]
fn text_on_paper_reads_in_dark() {
    assert!(contrast("dark", "text", "paper") >= 4.5);
}

#[test]
fn muted_on_paper_reads_in_dark() {
    assert!(contrast("dark", "muted", "paper") >= 4.5);
}

#[test]
fn edge_on_paper_shows_in_dark() {
    assert!(contrast("dark", "edge", "paper") >= 3.0);
}

#[test]
fn focus_on_paper_shows_in_dark() {
    assert!(contrast("dark", "focus", "paper") >= 3.0);
}

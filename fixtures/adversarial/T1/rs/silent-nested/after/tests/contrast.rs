use demo::contrast;
use rstest::rstest;

#[rstest]
#[case("text", "paper")]
#[case("muted", "paper")]
fn text_reads(#[values("light", "dark")] theme: &str, #[case] fg: &str, #[case] bg: &str) {
    assert!(contrast(theme, fg, bg) >= 4.5);
}

#[rstest]
#[case("edge", "paper")]
#[case("focus", "paper")]
fn mark_shows(#[values("light", "dark")] theme: &str, #[case] mark: &str, #[case] ground: &str) {
    assert!(contrast(theme, mark, ground) >= 3.0);
}

use demo::banner::{banner, Missing};

#[test]
fn a_banner_is_a_paragraph() {
    assert!(banner(&Missing::new("x")).starts_with("<p>"));
}

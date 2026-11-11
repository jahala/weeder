use demo::banner::{banner, Missing};

#[test]
fn a_banner_is_a_paragraph() {
    assert!(banner(&Missing::new("x")).starts_with("<p>"));
}

#[test]
fn wraps_the_error_in_a_paragraph() {
    let error = Missing::new("workspace not found");
    assert_eq!(banner(&error), "<p>workspace not found</p>");
}

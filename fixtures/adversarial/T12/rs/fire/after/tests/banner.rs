use demo::banner::{banner, Missing};

#[test]
fn a_banner_is_a_paragraph() {
    assert!(banner(&Missing::new("x")).starts_with("<p>"));
}

#[test]
fn shows_the_error_it_was_given() {
    let error = Missing::new("workspace not found");
    assert!(banner(&error).contains("workspace not found"));
}

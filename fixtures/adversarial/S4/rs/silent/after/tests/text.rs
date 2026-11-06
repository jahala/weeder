use fixture::text::slugify;

#[test]
fn slugify_joins_the_words_with_a_dash() {
    assert_eq!(slugify(" A b "), "a-b");
}

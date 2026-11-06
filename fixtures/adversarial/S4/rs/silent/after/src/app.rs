use crate::text;

pub fn title(input: &str) -> &str {
    text::trim(input)
}

pub fn link(input: &str) -> String {
    format!("/posts/{}", text::slugify(input))
}

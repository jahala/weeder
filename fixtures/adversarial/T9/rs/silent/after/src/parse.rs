pub fn parse(input: &str) -> Result<u32, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("empty input".to_string());
    }
    trimmed.parse().map_err(|_| format!("not a number: {trimmed}"))
}

pub fn normalise(input: &str) -> String {
    input.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trims_to(input: &str, expected: &str) {
        assert_eq!(normalise(input), expected);
    }

    #[test]
    fn reads_a_number() {
        assert_eq!(parse("42"), Ok(42));
    }

    #[test]
    #[should_panic(expected = "empty input")]
    fn refuses_an_empty_string() {
        parse("").unwrap();
    }

    #[test]
    fn reads_a_padded_number() -> Result<(), String> {
        parse(" 7 ")?;
        Ok(())
    }

    #[test]
    fn trims_a_padded_number() {
        trims_to(" 7 ", "7");
    }
}

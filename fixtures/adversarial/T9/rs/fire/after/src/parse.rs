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

    #[test]
    fn reads_a_number() {
        assert_eq!(parse("42"), Ok(42));
    }

    #[test]
    fn trims_a_padded_number() {
        normalise(" 7 ");
    }
}

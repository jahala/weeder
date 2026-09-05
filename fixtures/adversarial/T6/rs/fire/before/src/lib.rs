#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    Separator,
}

pub fn parse(input: &str) -> Result<Vec<&str>, Error> {
    if input.is_empty() {
        return Err(Error::Empty);
    }
    if input.contains(";;") {
        return Err(Error::Separator);
    }
    Ok(input.split(';').collect())
}

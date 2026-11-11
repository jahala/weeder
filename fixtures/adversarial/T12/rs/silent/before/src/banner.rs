pub struct Missing {
    what: String,
}

impl Missing {
    pub fn new(what: &str) -> Missing {
        Missing {
            what: what.to_string(),
        }
    }
}

pub fn banner(error: &Missing) -> String {
    format!("<p>{}</p>", error.what)
}

use crate::seams::git::read;

pub struct Finding {
    pub rule: String,
    pub path: String,
}

pub fn render(finding: &Finding) -> String {
    format!("{} {} {}", finding.rule, finding.path, read(&finding.path))
}

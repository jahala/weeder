pub fn summarise(rows: &[String]) -> String {
    rows.join(", ")
}

pub fn total(rows: &[u32]) -> u32 {
    rows.iter().sum()
}

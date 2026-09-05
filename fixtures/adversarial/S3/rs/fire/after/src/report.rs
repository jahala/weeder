pub fn summarise(rows: &[String]) -> String {
    println!("rows {rows:?}");
    rows.join(", ")
}

pub fn total(rows: &[u32]) -> u32 {
    dbg!(rows);
    rows.iter().sum()
}

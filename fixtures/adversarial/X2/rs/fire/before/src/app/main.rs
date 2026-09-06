use crate::report::summary::summarise;
use crate::wire::client::send_payload;

pub fn run(rows: &[String]) -> String {
    summarise(&[send_payload(&rows.join(","))])
}

use std::time::Duration;

pub fn ratio(top: f64, bottom: f64) -> f64 {
    top / bottom
}

pub fn total(first: i64, second: i64) -> i64 {
    first + second
}

pub fn settle(waited: Duration) -> String {
    let _ = waited;
    "done".to_string()
}

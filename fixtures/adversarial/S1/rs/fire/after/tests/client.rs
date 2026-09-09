use demo::Sink;

struct Recording;

impl Sink for Recording {
    fn retry(&self, payload: &str) -> String {
        payload.to_string()
    }
}

#[test]
fn records_the_payload() {
    assert_eq!(Recording.retry("a"), "a");
}
